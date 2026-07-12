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
   if relevant, final gate pass. Then the Volence checkpoint (merge).
5. **Optimize (Volence-ratified 2026-07-09, tranche-3 packet review)** — POST-MERGE: the
   tranche's reads-wrong list plus anything later retrospects send back to already-ported
   files. Byte-CHANGING by definition, so each lands as its own commit re-gated against a
   REBUILT reference ROM (PROVENANCE pins re-baseline). The loop's guarantee becomes: going
   through a tranche leaves the touched files at the campaign's latest quality bar, not just
   its latest byte-copy.

**THE STEP-2 CHECKLIST (standing, Volence-ratified 2026-07-09 — apply to EVERY file in every
tranche's modernize pass; the step-3 retrospect reviews the checklist itself for gaps):**
- **Contracts:** every FINISHED proc declares its register contract — `clobbers(...)` (source
  the .asm header comments AND the actual write set; outputs count as clobbers until an
  outs-annotation exists), `clobbers()` for a verified no-effect proc (Volence ruling
  2026-07-09; opt-in — absence stays legal mid-port), or `preserves(...)` (S2-D6b) on every
  movem-save/restore proc; `falls_into` wherever a proc falls through.
- **Types:** every `data` item carries its true type (the length IS the size check); prelude
  newtypes where a value is domain-typed (Angle, VramTile, …) — erasing, byte-neutral;
  `bitfield`/`enum` types where a flag/id byte buys real checking; `sizeof`/`offsetof` over
  magic numbers.
- **Constants:** no bare magic number where a named constant exists (`use engine.constants`);
  every mirrored cross-seam value gets its `ensure(extern("X") == X)` drift guard.
- **Control flow:** `jbra`/`jbsr` for label transfers; bare `jmp`/`jsr` only for computed
  targets; conditionals UNSIZED (Volence ruling 2026-07-09 — the assembler relaxes `.s`/`.w`
  by reach; explicit sizes only under `@as_compat`; `jbcc` still deliberately absent).
- **Guards & tests:** every hand-maintained invariant living in a comment becomes an
  `ensure`/`ensure_fatal` (item or statement position); cross-seam facts via `extern()`;
  `comptime test` beside every comptime fn.
- **Compat:** `@as_compat` removal evaluated per file with gate evidence (remove when inert).
- **Style:** strict bracket nesting; ruled banners; comments describe function; binary
  `%`-literals for bit masks where clearer (taste, per file); header In/Out/Clobbers comments
  must not contradict the attributes (attribute is authoritative, comment explains meaning).

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
  doc line owed in the .emp EA docs. — CLOSED tranche 3 (the reserved-token consequence documented on `pc_rel_shape`, eval/asm.rs).
- [tranche 2 T1 review, 2026-07-09] **PcRel range-check errors name distance+section but not
  the target SYMBOL** (sigil-link lib.rs ~482/498) — house style shared with bra/bsr messages;
  a cross-section disp8 target is almost always out of range, so the symbol name would pay.
  Repo-wide message-quality item. — CLOSED tranche 3 (all three 68k PcRel kinds + the zero-disp escape message now name the target symbol; pinned by `pcrel_out_of_range_messages_name_the_target_symbol`).
- [tranche 2 T1 review, 2026-07-09] **abs.l as an .emp DESTINATION is unsupported**
  (`move.w x, ($abs).l` → "indirect base must be a register") — pre-existing, surfaced by an
  adversarial probe; will matter for some future port (VDP register writes spell this). —
  SHIPPED 2026-07-09 (Volence ratified at the packet review): explicit-width `(expr).w`/
  `(expr).l` absolutes, BOTH positions — comptime-int addresses pin their bytes at lower
  (with asl's abs.w sign-extension window validated), symbolic addresses pin the WIDTH and
  defer as ONE fixed-width fixup (no RelaxAbsSym pair). Coexists cleanly with the bare-symbol
  idiom, which stays the new-style default (relaxes via the width rule to the same-or-smaller
  encoding). Ride-along hardening: `(a0).w` (sized register indirect — not a 68000 form) now
  rejects instead of silently dropping the suffix.
- [tranche 2 T1 review, 2026-07-09] **`Owner.label(pc)` multi-segment pc-rel target untested**
  — path shared verbatim with tested branch resolution; one-line test owed. — CLOSED tranche 3 (`owner_label_pcrel_target_resolves`, pcrel_port.rs; passed first run, a pin not a fix).
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
- [tranche-2 retrospect follow-up (Volence), 2026-07-09] **`clobbers(...)` annotations MISSED
  in step 2** — controllers.asm's "Clobbers: d0-d1, a0" and math.asm's clobber notes were
  carried as comments, not as the existing `clobbers(...)` proc attribute. Byte-neutral;
  land in tranche 3's step 2 (incl. hblank's dispatch proc alongside its @as_compat removal).
  — CLOSED tranche 3 step 2 (clobbers on Read_Controllers/GetSineCosine + all three new-port procs; hblank got `preserves`, being movem-preserved).
- [tranche-2 retrospect follow-up (Volence), 2026-07-09] **Pull the SYNTACTIC slice of
  S2-D6(b) `preserves(...)` forward** — declared `preserves(d0-d1/a0)` verified against the
  literal `movem` push/pop pair (HBlank_Dispatch is the poster child). The full S2-D6
  register-contract batch stays gated on the dataflow pass, but the movem-pair check is
  simple pattern matching, the campaign keeps producing exactly this shape, and Volence asked
  for it by name. Candidate for tranche 3 step 4 (a recorded decision per A-Spec2.3 — it adds
  a proc-attribute surface). — SHIPPED tranche 3 (pulled FORWARD to the tranche opening per the step-2-apex rule; D2.32 recorded per A-Spec2.3; HBlank_Dispatch annotated).
- [step-2 checklist audit of hblank/controllers/math, 2026-07-09] Beyond the clobbers miss:
  (a) **hblank's dispatch proc clobbers NOTHING** (movem-preserved) — its correct annotation
  is `preserves`, which waits on the S2-D6(b) slice; annotate when it ships. (b) **math's
  `GetSineCosine` takes an ANGLE in d0** — the Angle prelude newtype ([[emp-sonic-newtype-
  candidates]]) wants to type that param, but whether proc params support DATA registers
  (`d0: Angle`) vs only address registers (`a0: *Sst`) needs a check; if unsupported, that's
  a v1.1 candidate (typed data-register params). (c) controllers' `#$3F`/`#$30` masks are
  binary-literal candidates (taste; step-2 judgment). (d) idea jotted: comptime CONTENT
  asserts on embedded blobs (e.g. sine table's sin(0)=0 / +$40 overlap invariants checked
  against the embed bytes at build time) — needs comptime byte-indexing of Data values;
  check support, else v1.1 candidate. — RESOLVED tranche 3 (a: preserves shipped + applied; b/d: checked, see the tranche-3 entries below; c: applied). — (a: tranche 3 w/ preserves; b/d: check-then-
  ledger; c: taste).

- [tranche 3, 2026-07-09] **Typed data-register params ALREADY WORK** (checklist-audit item (b)
  resolved): `proc GetSC (d0: Angle)` with `newtype Angle = u8` parses, lowers, and emits
  byte-identically (probed through the real CLI). Deliberately NOT applied to math.emp this
  tranche: the Angle newtype belongs to the engine-side type surface that construct walk #3
  (the Sonic newtype set vs player physics — Volence driving) is scheduled to design; typing
  one param ahead of that walk would front-run the naming/ownership decisions. — RESOLVED
  (support confirmed); application rides construct walk #3.
- [tranche 3, 2026-07-09] **Comptime byte-indexing of `Data` values does NOT exist** (checklist-
  audit item (d) resolved): `ensure(embed("f.bin")[0] == 0, ...)` fails to parse (`expected
  \`)\`, found LBracket`). Blocks comptime CONTENT asserts on embedded blobs (the sine-table
  sin(0)=0 / +$40-overlap invariants). v1.1 candidate: index + `.len` on `Data` in comptime
  exprs — pays at every embed with a checkable invariant. — RATIFIED IN 2026-07-09 (Volence,
  tranche-3 packet review); builds together with typed Data views (one plumbing item, below)
  at tranche 4's opening; A-Spec2.3 decision record rides the build.
- [tranche 3, 2026-07-09] **Clobber lint: `dbcc`'s first-operand write remains the recorded
  blind spot** (pre-existing TODO(S2-D6) in lower/proc.rs) — vdp_init's two `dbf` loops write
  d0/d3, which its `clobbers` declares anyway; noting the tranche relied on declaration
  discipline, not the lint, for those two. — OPEN (S2-D6 territory, unchanged).
- [tranche 3, 2026-07-09] **Struct-equ export exports EVERY struct's `_len` + field offsets**
  (the VDP_Shadow_len enabler). Surplus symbols in the link table are harmless today (the
  convsym allowlists are inclusive), but if link-symbol-table noise ever matters (Spec 4 debug
  info?), an export filter is the knob. — jotted.
- [tranche 3 retrospect, 2026-07-09] **Checklist gap for rulings: the no-effect proc.**
  `HBlank_Null` (bare `rts`) carries neither `clobbers` nor `preserves` — the checklist says
  "clobbers on every proc" but an EMPTY `clobbers()` is currently unparseable-by-intent and
  "no contract declared" is indistinguishable from "contract: touches nothing". Options:
  (i) bare stays legal, absence = no contract (today's shape); (ii) allow explicit empty
  `clobbers()` meaning "touches nothing" (the lint then flags ANY register write). Recommend
  (ii) at low priority — it makes the strongest contract expressible — but it needs a Volence
  ruling + checklist amendment. — RULED 2026-07-09 (Volence, tranche-3 packet review):
  option (ii) ADOPTED as OPT-IN — absence stays legal (half-ported/@as_compat files), empty
  `clobbers()` means "touches nothing" (lint flags any write), and the step-2 checklist is
  amended: every FINISHED proc declares its register contract (clobbers(...) / clobbers() /
  preserves(...)). — SHIPPED same day (pulled into the tranche at Volence's "let's do" —
  `clobbers` is now `Option<Vec>`, `Some([])` = the verified touches-nothing contract, the
  lint flags any write inside; HBlank_Null annotated).
- [tranche 3 code-sense review, 2026-07-09] **`~mask` byte-width ceremony tax**:
  `andi.b #~(BUTTON_LEFT|BUTTON_RIGHT)&$FF, d0` (controllers.emp) — the `&$FF` exists only
  because comptime `~` is untyped-width; a byte-width operand position could plausibly
  auto-truncate a comptime complement (loudly range-checked otherwise). Jot-don't-implement:
  needs a decision on silent-vs-explicit truncation semantics (tenet: no silent wrong bytes).
  — RESOLVED 2026-07-09 (post-ruling probe): NO language change was ever needed —
  `#~(BUTTON_LEFT|BUTTON_RIGHT)` = -13 already fits the signed imm8 window and emits the
  identical $F3 (CLI-probed). The `&$FF` was INHERITED AS SPELLING (controllers.asm needs it
  under asl); dropped from controllers.emp, byte-gates green. The safe-truncation design
  question is moot for this case; reopen only if a genuinely-out-of-window complement shows up.
- [tranche 3 code-sense review, 2026-07-09] **Typed word-table embeds**: `Sine_Table:
  [u8; $280] = embed(...)` describes a WORD table as bytes; `[i16; 320]` with big-endian
  byte-identity would be the honest type if/when `embed` learns typed element views. Pairs
  with the comptime-Data-indexing candidate (content asserts want typed reads too). —
  RATIFIED IN 2026-07-09 (Volence, tranche-3 packet review); one work item with the indexing
  candidate, tranche 4's opening.
- [tranche 3 branch review, 2026-07-09] **`ifndef`-guarded equs/structs export NO equ-syms in
  the converged pass** (pre-existing for Task B1 `equ` export; tranche 3 widened the mechanism
  to struct symbols): pass 0 defines the guard symbol and exports, the converged pass sees the
  guard defined and SKIPS the block, and only the converged module is returned — bytes correct,
  `equ_syms` empty. Any `extern("X")` on such a symbol fails the link with a misleading
  "unresolved symbol" even though the front-end folds it fine. Harmless today (aeon's
  constants/structs are unguarded), but a conventional include guard around a constants file
  would silently break every drift guard reading it. — CLOSED tranche 3 (Volence's call at the
  packet review): the run loop carries the ever-exported set across passes and re-attaches
  missing exports from the CONVERGED env (values authoritative — a forward-ref-dependent equ
  gets its final value); pinned by `ifndef_guarded_equs_and_structs_still_export`.

- [tranche-3 packet review (Volence), 2026-07-09] **STEP 5 RATIFIED** — optimization is now
  the loop's fifth step (post-merge, re-gated, re-baselined; see the loop description above).
  Tranche 3's reads-wrong list is its first work queue.
- [tranche-3 packet review (Volence), 2026-07-09] **Unsized-conditional taste call OPEN** —
  D2.18's unsized `Bcc` relaxation exists; the checklist currently keeps explicit `.s`/`.w`
  on conditionals in ported files. Dropping suffixes is byte-neutral (relaxation picks the
  same minimal size). Volence to rule: keep classic explicit sizes vs assembler-managed
  unsized. `jbcc` (trampoline-expanding unlimited-reach conditional) stays deferred either
  way (tenet 3). — RULED 2026-07-09 (Volence): UNSIZED adopted for new-style files — the
  assembler picks `.s`/`.w` by reach; explicit sizes remain only under `@as_compat`. All six
  ported files swept (bne/blt/bgt/beq ×8), byte-gates green (relaxation picks the identical
  sizes). Checklist amended.

- [tranche-4 recon (Volence's naming question), 2026-07-10] **`plantbadmaps` is not in the
  build** — zero hits in s4.lst; `data/sprites/plantbadmaps/` (art.bin + mappings.bin +
  anims.asm + sprite.json) is a parked editor export whose object was never wired in. Two
  consequences: (a) it CANNOT be a port target (no reference window to byte-gate — the
  kickoff's data-quick-win list was wrong about it; `sonic_anims.asm` takes its tranche-4
  slot); (b) the entity RENAME is free right now — nothing consumes the name. Naming finding
  (Volence): the sprite entity is named after ONE of its assets ("plant badnik MAPPINGS", a
  donor-repo label habit — sonic_hack's MapUnc_PlantBad class), and the bundle dir inherits
  it. Proposal pending Volence's pick: entity → `pitcher_plant`; per-sprite bundle dirs named
  for the ENTITY with generic member names. "mappings" stays the term for piece tables
  themselves (community-standard). — OPEN (naming ruling + free rename window).

- [step-5 execution, 2026-07-10] **STEP 5 QUEUE COMPLETE** — both ratified reads-wrong items
  landed as post-merge commit pairs (aeon `4352a40`+sigil `bc55333`; aeon `9eb2101`+sigil
  `ae48ac7`), each: lockstep .emp+twin edit → both shapes rebuilt → neutrality sha256 ×3 →
  full re-pin → strict 1895/0 → oracle boot-check. The marginal items (Flush shift-out loop,
  controllers P1/P2 pointer dedup) stay SKIPPED per the handoff (no VBlank-headroom pressure).
  Two findings for the record:
  (a) **"clobbers shrink to d0/d1" was review shorthand for the LOCAL write set** — the
  caller-facing attribute must stay `clobbers(d0,d1,d2,d3,a0)`: d2/a0 are still trashed via
  the Tile_Cache_GetCollision TAIL CALL, and the precedent (the original attribute carried
  d3/a0 "via callee") makes the attribute transitive. Shrinking it would let a caller keep a
  live d2 across the call. Landed with the full set + a header comment spelling the split —
  flag at the next packet review in case Volence intended attribute-as-local-writes instead.
  (b) **`clr.l` is a size win, not a speed win** (Volence asked mid-session): on the 68000,
  `moveq #0,d0` + `move.l d0,(abs.w)` = 4+16 = 20 cycles (3 reads/2 writes total) and
  `clr.l (abs.w)` = 20 cycles (3/2 — clr does a dummy read of the destination). Identical
  cycles AND bus profile; the win is 2 bytes per site + no scratch register. The trade
  REVERSES with N>1 zero-writes sharing one moveq (each extra move.l is 16 vs clr's 20) —
  vdp_init has one site per proc, so clr.l is right here. (68010+ makes clr a pure write;
  the I/O read-hazard caveat stands on 68000 — comment carried in both files.)
- [step-5 re-pin sweep, 2026-07-10] **RE-PIN HAZARD: per-byte address literals are invisible
  to hex-string sweeps** — bare-name proofs encode addresses as `[0x00, 0x00, 0x22, 0x7E]`
  and split words (`0x24, 0x68`), which a `227E`/`2468` grep can't see; three test files
  tripped the strict gate before being caught (hblank_port, math_port bare-name pins,
  vdp_init_port's Flush second-proc offset). A future re-baseline should either grep both
  spellings or (better) derive bare-name expectations from the map constants instead of
  literals — jotted as a small-opens candidate. — OPEN

- [tranche-4 opening build (D2.33), 2026-07-10] **Postfix `.field` size-letter carve-out** —
  postfix field access off non-path bases (`embed(...).len`) never consumes `b`/`w`/`l`/`s`,
  or it would swallow asm operand size suffixes (`timer(a0).l` — caught by the existing
  parser_bodies pin during TDD). Same accepted trade as the `split_size_suffix` operand rule;
  a comptime struct field genuinely so named needs a const binding first. Also recorded in the
  D2.33 spec row. Method calls on expression results (`f(x).map(g)`) remain unsupported with a
  steering diagnostic — jot for a future postfix-call increment if real files want chains. — SHIPPED (the carve-out) / OPEN (postfix method calls)
- [tranche-4 opening build (D2.33), 2026-07-10] **`[index.uncommitted-byte]` is defensive
  depth** — every expr-position `Data` source today (embed/bytes/byte/++) builds raw `Bytes`
  cells, so the eval-path diagnostic can't fire yet; the gate logic (`DataBuf::byte_at`) is
  unit-tested directly (width-1 scalars read as two's-complement bytes, multi-byte scalars and
  SymRef/RelOffset/Expr cells refuse). Becomes reachable the day a Data-monoid builtin emits
  structured cells in expr position. — SHIPPED (noted for the retrospect)

- [D2.33 review, 2026-07-10] Review findings triage: **C1 huge-index usize wrap FIXED**
  (bounds compare in i128 + tests), **I2 poisoned-view-element cascade FIXED** (single
  diagnostic), **M5 no_struct_lit save/restore in index brackets FIXED**, **M3 new
  diagnostics tagged** ([index.type]/[index.base]). Jotted for rulings/later: **I1** — a
  NON-array annotation over raw Data (`data X: u16le = embed(...)`) bypasses the view
  policing (pre-existing acceptance; police scalar/struct annotations too, or bless?);
  **M6** — asm operands route through expr(), so `move.w Tbl[2], d0` now PARSES as a
  comptime index (fell out naturally, in-spirit but not in D2.33's ratified text — confirm
  or fence); **M2** — no steering when postfix hits the b/w/l/s carve-out in comptime
  context; **M4** — no integration test pins indexing inside an asm splice; **M1** — the
  method-call steer leaves the call tokens unconsumed (steer + one cascade line). — OPEN

- [tranche-4 port recon, 2026-07-10] **Target list re-scoped at recon (the handoff's (*)
  hedge earned its keep):** (a) `vram_bases.asm` is NOT IN THE BUILD — zero s4.lst hits,
  a parked editor export under `data/editor/ojz/act1/export/` (the plantbadmaps class);
  no reference window to byte-gate, DROPPED. The REVERSE-SEAM proof it was meant to carry
  (.emp equ export -> AS reads) needs a replacement carrier — candidate: a small in-build
  equ/config file, to be picked when the seam work runs. (b) `ojz_act_pool.asm` IS in the
  build (15 lst hits) but is AUTO-GENERATED BY build.sh into the uncommitted
  `data/generated/` tree — porting it is generator-emits-.emp mechanics (the
  "reproducibility own session" ledger item), not a straight port; RE-SCOPED to a packet
  ask. (c) `particle_anims.asm` (15 ln) + `sonic_anims.asm` (83 ln): committed, in-build,
  exactly the offsets+inline-bodies shapes — the tranche's two LIVE port targets. — OPEN
  (two packet asks ride the checkpoint)

- [tranche-4 port recon, addendum (Volence's catch), 2026-07-10] **The `ojz_act_pool` slot
  is really `act_descriptor.asm`** — the handoff's "align 2 ×3 between BINCLUDEs + dc.l
  table" describes the GENERATED wrapper, but the committed, portable file in that
  neighborhood is `data/levels/ojz/act1/act_descriptor.asm` (254 ln): the Act descriptor
  struct written as raw dc rows (→ struct-typed `data` item, the type IS the Act_len
  guard), three hand-maintained if/error asserts (→ `ensure`s), the 3×3 section tables,
  and the generated includes staying AS-side with `OJZ_Act_Pool_PageTable`/
  `OJZ_ACT_POOL_PAGES` crossing the seam as externs — recovering seam coverage the
  dropped `vram_bases` was meant to provide. ADOPTED as the tranche-4 third target
  (recon-correction precedent: plantbadmaps→sonic_anims); ratification rides the
  checkpoint packet. — OPEN

- [tranche-4 ports #1/#2 (overnight), 2026-07-10] **Data-file ports are BORN modern** —
  for offsets-shaped data files, step 1 (transcribe) and step 2 (modernize) collapse: the
  only .emp spelling of `dc.w Target-Base` IS the offsets construct, the guards are the
  modern form, and there is no verbatim-instruction dimension. The loop's steps stay
  meaningful for CODE files; jot for the retrospect (the checklist could say so). — OPEN
- [tranche-4 port #1, 2026-07-10] **imm32 deferral extended to d16(An) destinations** —
  `move.l #Ani_Particle, SST_anim_table(a0)` (the anim-table write EVERY spawn template
  uses) hard-errored cross-seam; the symbolic-operands port's deferred #Sym-immediate
  item got its real consumer. Shipped with the port (offset-2 proof + 217C encoding
  test). Remaining imm shapes (other destinations) still fall to the eager path —
  extend on demand. — SHIPPED
- [tranche-4 ports #1/#2, 2026-07-10] **AF_*/DUR_DYNAMIC constants-twin growth** — both
  anims ports carry local const mirrors + extern drift guards (AF_END/AF_BACK/AF_DELETE/
  DUR_DYNAMIC; truth in animate.asm + engine constants.asm). When a third consumer
  appears, grow engine.constants' twin an animation block per the twin growth pattern. — OPEN
- [tranche-4 #3 prep, 2026-07-10] **act_descriptor design note written**
  (notes/2026-07-10-act-descriptor-design.md — Volence's "we'll have a lot of those"
  ask): Tier 1+2 (typed Act/Sec literals + a shared validating act() constructor) is the
  recommended port shape; Tier 3 (mapped section grids via computed string labels) needs
  ONE small increment (computed-name extern()); Tier 4 (acts via import(), generators
  stop emitting .asm) is the post-campaign direction and resolves the ojz_act_pool
  generator question. Ratification rides the checkpoint packet. — OPEN

- [Volence's ask, 2026-07-10] **Twin-scaffolding kill list CREATED** —
  notes/twin-scaffolding-kill-list.md: every temporary seam mirror (local const twins, the
  engine.constants twin, the ANIM ordinal guards, the AS twins + gate pins themselves) gets
  a row with its kill condition (ownership flip / twin consolidation / Spec-5 deletion).
  Cadence: ports that add mirrors add rows; checkpoint packets review the list;
  campaign-end sweep closes survivors. — SHIPPED (the practice)

- [D2.33/D2.34 rulings (Volence), 2026-07-10] **Three rulings landed same-day:** (a) I1 —
  scalar `le` annotations over raw Data POLICED (same [data.view-le] as arrays; the
  hypothetical LE-blob case designs its real answer if it ever appears); (b) M6 — SPLIT:
  `#Tbl[i]` immediate BLESSED (pure comptime, sizeof's class), bare `Tbl[i]` address
  operand FENCED with steering ([asm.index-operand] — one typo from `Tbl+2`, not classic
  instruction syntax); (c) the ANIM-ordinal reverse-seam flip, SCOPED — stage 1 proven
  (reverse_seam_ordinals.rs: .emp equ-exports ordinals, AS consumes; zero new machinery),
  stage 2 (config-block deletion) parked at Spec 5. — SHIPPED
- [reverse-seam proof, 2026-07-10] **imm deferral lacks `.b`/`.w` widths** — the real
  player shape `move.b #ANIM_WALK, SST_anim(a0)` hard-errors cross-seam (only `.l`
  defers). Blocks kill-list row 4 stage 2; extend try_defer_long_imm's family when the
  flip (or any port) demands it. — OPEN

- [tranche-5 port #1 (game_loop), 2026-07-10] **Statement-position comptime `if` SHIPPED**
  (H1's carrier: parser `asm_if`, `AsmStmt::If`, recursive label scope, script-body
  label-under-if refusal; 12 tests). Two edges deliberately NOT built: (a) a `yield`
  inside a comptime-if branch in a SCRIPT body parses as an unknown mnemonic (a
  `ScriptStmt` can't nest in an `AsmStmt` branch by type) — the error is loud but
  unhelpful; teach a steering diagnostic if a script port ever hits it. (b) the
  AS-`ifdef`-presence vs .emp-value-define convention (AS omits the define, .emp
  passes 0) is harness-mapped per call site — fine while the harness owns both
  sides; the sigil CLI's `-D` story should NAME the convention when sound-off
  builds go end-to-end through the CLI. — OPEN (both jots)
- [tranche-5 port #1 (game_loop), 2026-07-10] **`extern-macro` / game-contract-hook
  construct NOT built** (H2 option (c) rejected — no demand): sonic4's gameDebugTick
  body is a plain `jsr`, mirrored under comptime-if (kill-list row 9). The demand
  moment is the first game-contract macro with a NON-TRIVIAL body reaching a port. — OPEN
- [tranche-5 review, 2026-07-10] **sigil-link does not range-check pc-rel16 fixups** — a
  `bsr.w`/`bra.w` whose resolved displacement exceeds ±32K wraps mod 2^16 and links
  silently (surfaced by the port tests' far-carrier consumer proofs; game_loop's
  consumer moved in-range, collision_lookup's inherited far carrier left as-is).
  Add a loud link error when a PcRelDisp16 fixup overflows i16. — OPEN
- [tranche-5 port #2 (sound_api), 2026-07-10] **Three demanded features SHIPPED:**
  (a) abs-sym ext-word fence relaxed to POSITIONAL (imm/d16 BEFORE the sym operand
  OK — its ext words precede the abs field; AFTER stays fenced); (b) emp-side
  link-time imm32 (`ImmLink` → Value32Be at 2, `#extern(...)`/equ-alias spelling —
  the AS `try_defer_long_imm` mirror; `.l` ONLY, so the `.b`/`.w` row above is
  UNCHANGED and still blocks kill-row-4 stage 2); (c) `sr`/`ccr` operands
  (register-class words, the interrupt-mask idiom). Also NOTE: a provisional
  here() in `.l` IMMEDIATE position now defers as an imm-link instead of the
  blanket [here.provisional] rejection (sound by the D2.23 model — the fixup
  resolves at final positions; bankid()-derived values KEEP their rejection).
  Spec addendum wanted at the checkpoint. Message-tier detail for that pass: a
  provisional here() in a `.b`/`.w` immediate now surfaces the generic
  `[lower.imm-link] needs .l` steering instead of D-H.2's `[here.provisional]`
  wording (still loud; fold the case into the addendum). — SHIPPED (jot the spec pass)
- [tranche-5 port #2 (sound_api), 2026-07-10] **Reads-wrong list (post-merge step-5
  candidates, byte-DIFFERENT so post-port):** of the EIGHT `bra.w` tail-calls
  (6→PostByte, 2→PlaySFX), only Sound_Ping/Sound_PlaySample's two are in `.s`
  reach (disp ≈ −92/−102) — jbra would keep the other six `.w`, so the real
  saving is −4 B, not the naive −16 (review-corrected). Also `moveq #0,d1` +
  reload in Sound_PlaySFX's dedup path reads clunky but is cycle-honest —
  leave it. — OPEN
- [tranche-5 whole-branch review, 2026-07-10] **F1 FIXED at the ISA level:** move
  to/from `sr` is now word-only-policed in `encode_move_sr` (both frontends heal at
  once) — the old encoder keyed the imm ext width to `inst.size`, so
  `move.l #$2700, sr` silently emitted `sr := $0000` + `$2700`-as-opcode. Pinned by
  `move_sr_is_word_only`. Remaining legal-68000 SR/CCR forms NOT modeled (all refuse
  loud): `move <ea>, ccr` (44C0), `andi/ori/eori #imm, sr`, `eori #imm, ccr`
  (`move.w ccr, d0` refusing is CORRECT — 68010+). Build them when a port demands. — PART-SHIPPED
- [tranche-5 whole-branch review, 2026-07-10] **emp silently truncates out-of-range
  word immediates** (`move.w #$12345, d0` → `30 3C 23 45`) where the AS front-end
  errors — a pre-existing parity divergence in the generic `Imm` path (not
  tranche-5's; byte-gates can't see it since gated sources are in-range). Add the
  AS-matching range check. — OPEN
- [tranche-5 whole-branch review, 2026-07-10] **Minor jots:** a `todo!` in an
  UNCHOSEN comptime-if arm produces no [todo.present] (consistent with
  never-lowered, but a define-gated todo vanishes from the list — note-tier
  candidate); duplicate-label link errors expose the mangled `$m$f$x` name
  (cosmetic, unmangle in the renderer). — OPEN
- [tranche-5 step 5 (engine review), 2026-07-10] **No engine changes — recorded why:**
  game_loop is optimal for its design (RAM-dispatch loop, 18 B). sound_api: (a) the
  SR-mask is load-bearing in ALL builds (VBlank's DMA stopZ80, not just the DEBUG
  mirror — comment fixed both twins); (b) Sound_PlayMusic's `>>15` via two lsr.l
  could be ~34 cycles cheaper (`add.l dN,dN` + `swap`) — REJECTED: once-per-song
  cold path, clarity wins (recorded so nobody re-derives); (c) the PlaySFX ring
  dedup was already judged cycle-honest. — CLOSED
- [tranche-5 step 2 retrospect, 2026-07-10] **stop_z80()/start_z80() comptime-fn
  templates proven** (the .emp answer to AS macros, hygienic per-site labels) —
  currently sound_api-local; when a SECOND file wants them (sound_debug port is the
  likely demand), lift into a shared engine-macros .emp module (`use`-imported
  Code-returning fns). Demand-gated, don't pre-build. — OPEN
- [tranche-5 retrospect, Volence reading Sound_PostByte, 2026-07-10] **`preserves(sr)`
  is unspellable** — the contract vocabulary is d0-a7 only, so the SR save/mask/
  restore idiom's contract lives in a comment ("Clobbers: SR restored") the compiler
  can't check. This is S2-D7's first concrete demand site (machine-state contracts:
  SR/CCR liveness, stack-delta, stopZ80/disableInts PAIRING lints — Sound_PostByte
  exhibits all three balanced pairs in one proc). Surface ask: accept `sr` (and
  `ccr`?) in preserves()/clobbers() lists as DECLARED contract, verification riding
  the S2-D7 dataflow pass; don't ship the spelling without at least the
  save/restore-balance heuristic, or it's documentation cosplaying as a check.
  → SHIPPED same-day (Volence's go): `preserves(sr)` + the static-order balance check
  ([proc.preserves-sr-unbalanced], error-tier — declared contracts are checked),
  `clobbers(sr)`, the [proc.sr-undeclared] warning (contract'd procs only, @as_compat
  silenced), ccr steered to S2-D7 proper (flag liveness = dataflow, refused with the
  pointer). Sound_PostByte/Init/PlayMusic/DrainSfxRing declare it; 8 tests.
  Path-sensitive save/restore stays S2-D7's dataflow half. — SHIPPED (slice)
- [preserves(sr) slice, 2026-07-10] **clobbers() entries are never validated** —
  `clobbers(d9)` or a typo'd name is silently accepted (it just never matches the
  lint's allowed-set lookup). Cheap fix: validate entries against the register
  vocabulary (+ `sr`) at the same site preserves validates. — OPEN
- [tranche 6, 2026-07-10] **Label references in comptime/immediate expressions do not
  exist** — a bare proc name in an instruction immediate is `unknown name` (the D-PP.3
  label-value fallback is confined to data initializers and call args). The objroutine
  store had to spell its OWN module's label through the link:
  `equ SOLID_ROUTINE_MAIN = extern("TestSolid_Main") - extern("ObjCodeBase")` — legal
  (the R3-flip precedent), but self-extern reads as ceremony. Ask: label values in imm
  exprs (→ LinkExpr), which also unlocks an expression-position `objroutine(label)`
  comptime helper. Every future object port hits this once per routine store. — OPEN
- [tranche 6, 2026-07-10] **equ names are link-global and unhygienic** — two modules
  declaring `equ OBJ_CODE_BASE = …` collide at link ("symbol redefined"), so
  module-local equs must carry manual name prefixes (SOLID_/PARTICLE_). Ask: mangle
  non-pub equs like non-pub procs (they are module-local by declaration), or at least
  say "equ" and name both modules in the collision diagnostic. — OPEN
- [tranche 6, 2026-07-10] **clobbers() takes no register ranges** — `clobbers(d0-d4/a1-a3)`
  is a parse error while `preserves(d0-d1/a0)` takes movem-reglists; the eight-register
  contract had to be spelled out comma-by-comma. Ask: accept the movem-reglist grammar in
  clobbers too (one grammar for both attributes). Third data point tranche 7:
  TouchResponse's whole-file contract spelled TWELVE registers comma-by-comma
  (`d0..d7/a0..a3`). — OPEN
- [tranche 6, 2026-07-10] **offsets-table labels can't be `use`-imported as values** —
  test_particle consumes `Ani_Particle` (an .emp offsets table in particle_anims.emp)
  via `equ ANI_PARTICLE = extern("Ani_Particle")` even though both sides are .emp. Ask:
  `use games.sonic4.particle_anims.{Ani_Particle}` importing the table label as a
  link-value name usable in imm position (pairs with the label-in-expr ask above). — OPEN
- [tranche 6, 2026-07-10] **AS-side deferred `dc.w` values use the UNSIGNED Value16Be
  window** (shipped this tranche for the objroutine consumer words): a NEGATIVE deferred
  difference fails the link loud where asl would truncate mod 2^16. Right call for bank
  offsets (totality); recorded as a deliberate asl divergence in the F5 truncation-parity
  family. Bank offsets ≥ $8000 stay representable (unsigned window) — the earlier
  signed-window worry applies only to the offsets-construct's RelWord16Be, unchanged. — RECORDED
- [tranche 6, 2026-07-10] **`.b` ImmLink still unbuilt** — the `.w` half shipped this
  tranche (Value16Be at offset 2); `.b` waits for its first consumer (kill-row-4 stage 2
  wants `.b`/`.w` BOTH — the `.w` half is now real). — OPEN (consumer-gated)
- [tranche 6, 2026-07-10] **sst.emp's 30 drift ensures are hand-written boilerplate** —
  a comptime iteration over a struct's fields (`for f in Sst.fields { ensure(...) }`)
  would collapse them to three lines. Reflection is a big hammer for one file; jotted as
  taste, NOT asked — revisit only if a second full-struct twin appears before structs.asm
  ports. — JOTTED
- [construct-walk #3, 2026-07-10] **Register OUTPUT typing does not exist** — proc
  params take types (`d0: Angle`, shipped for GetSineCosine this walk) but there is no
  out-annotation for register returns, so sin/cos's output contract lives in a comment.
  Ask: out-register contracts (pairs with the clobbers/preserves family). Ruling
  PRE-MADE for the first consumer: sin/cos return the BARE fixed<8,8> unit fraction —
  NOT Velocity (a scale factor becomes a velocity only when multiplied by a speed;
  Volence probed exactly this and the distinction held). — OPEN
- [tranche 7, 2026-07-10] **`comptime fn` register-arg DISTINCTNESS is contract-only** —
  `aabb_axis_test`'s doc contract says `stmp` MUST NOT alias `cdim`/`delt`, but the
  template splices whatever `Reg` args it is handed: passing `stmp == cdim` assembles
  CLEAN and emits silently-wrong code (tranche7_negative_probes.rs probe (f) pins this,
  matching the .inc's identical sharp edge — the AS macro can't check it either). Ask: a
  `distinct(a, b, ...)` param predicate on `comptime fn` (or an inferred no-alias check
  over splice sites) so the mis-instantiation fails loud instead of miscompiling. Pairs
  with the clobbers/preserves contract family. — OPEN
- [tranche 7, 2026-07-10] **bra.w jump tables are a THIRD dispatch-table encoding class** —
  collision's handler table is 13 `bra.w` entries indexed `jsr table(pc, type*4)`: the
  entry STRIDE is ABI, so the entries can never take the jbra idiom (relaxation would
  corrupt indexing — documented LOAD-BEARING at the table). The `dispatch` construct
  already owns `word_offsets`/`long_ptrs`; a `branch_table` encoding (fixed-width bra.w
  entries, stride guaranteed by construction) would make the constraint structural
  instead of a comment. Demand-gated on the next such table. — OPEN (consumer-gated)
- [tranche 7, 2026-07-10] **no local typed-register binding inside a proc body** —
  TouchResponse loads a2/a3 itself (no register-argument contract, so no `*Sst` params)
  and pays the qualified spelling `Sst.field(aN)` at THIRTEEN sites, while its handlers'
  `a2: *Sst` params get the bare form. Ask: a body-level binding (`let a2: *Sst` or
  similar) granting bare field access after a self-loaded pointer. — OPEN
- [tranche 7, 2026-07-10] **imported `pub comptime fn` bodies can't see their home
  module's private items** — F3 ships the param-only case (aabb's shape); a shared
  template referencing a home-module const fails LOUD naming the symbol. Fix direction:
  canonicalize home-module references at injection (rename-pass extension). Becomes real
  the first time a shared template wants a module-local constant. — OPEN (consumer-gated)
- [tranche 7, 2026-07-10] **PROCESS: fresh aeon worktrees silently build a different
  ROM** — the Aurora editor's per-section `.bin` working data (games/sonic4/data/editor/)
  is gitignored; without it the generators fall back to air-baseline collision and the
  ROM diverges 130KB with no error. Ask (aeon-side): build.sh should WARN (or fail under
  a strict flag) when editor data is absent; worktree setup docs should say "copy
  games/sonic4/data/editor/ in". — OPEN
- [tranche 7, 2026-07-10] **`sigil emp --root` prunes modules unreachable from the
  entry file** — a `pub equ`-only stub module not `use`d by anything never joins the
  link, so its exports can't satisfy externs (bit during smoke-testing; the harness's
  explicit module lists are unaffected). Possibly by design (manifest discovery vs
  link membership); jotted so the next single-program consumer knows. — JOTTED
- [tranche 7, 2026-07-10] **`Code ++ Code` shipped; label spaces stay per-fragment** —
  the Code monoid concat existed for Data only (spec §6.2 promised both); the aabb
  template's conditional head (`let head = if aliased { asm{} } else { asm{ move… } };
  head ++ asm{ …body… }`) demanded the Code arm. Semantics pinned: `++` composes ITEMS,
  each `asm { }` keeps its own hygiene scope — a cross-fragment label branch fails LOUD
  (unresolved symbol), never silent. Ask (consumer-gated): fn-call-scoped hygiene so a
  template can split a labeled region across fragments; until then keep shared labels in
  one fragment. — OPEN (consumer-gated)
- [tranche 7b, 2026-07-10] **bare const names in displacement position are CLOSED on
  typed base registers** — the interact-pointer fix needed to write the engine-owned
  `$4E` tail word off a `*Sst`-typed register. A bare const `interact_off(a2)` on a TYPED
  base register does NOT resolve the const — the displacement slot is closed to the
  struct's field namespace, so a typo'd field can never silently resolve to a same-named
  module const (correct totality). The sanctioned escape is the CALL-EXPR spelling
  `interact_off()(a2)` (comptime fn returning the int) — probe-verified. On UNTYPED
  registers a bare const in displacement position resolves fine (no field namespace to
  shadow it). Right call — the typed-register closure is the feature, not a gap. — RECORDED
- [tranche 7b, 2026-07-10] **operand splices are template-only** — F1's `{off}({reg})`
  operand-position splice parses inside a `comptime fn` asm block (aabb's `sub.w
  {boff}({breg}), {delt}` shape) but NOT in a plain proc body: `splices_allowed` gates
  the splice syntax to template contexts. So a proc that wants a spliced displacement
  reaches for the call-expr escape (row above) rather than a raw `{}` splice. Noted as a
  deliberate SCOPE boundary (splices belong to comptime-fn templates), not necessarily
  wrong — jotted so the next proc-body consumer knows the boundary before hitting it. — RECORDED
- [tranche 7 close, 2026-07-10] **DESIGN-READY: object-pool occupancy structure** —
  four per-frame sites walk FIXED capacity to find sparse occupancy (RunObjects'
  dynamic/system/effects sweeps incl. per-live distance culling; TouchResponse ×2
  players ~5.3k cycles measured; EntityWindow_DespawnObjects' 40-slot walk;
  RunObjects_Frozen cold). ~7-8k cycles/frame (~6% NTSC) of pure empty-slot tax at
  typical occupancy. Fix: ONE compact live-index array per pool, maintained at
  spawn/despawn (rare events — swap-remove), consumed by all four sites; the SAME
  structure is the registered-participants list opt-in object-vs-object collision
  needs (one build, two features). Rings already escaped this pattern at birth
  (dense ring buffer). HAZARDS: mutation-during-iteration (children/effects spawn
  MID-RunObjects-walk — needs append-only-during-frame + deferred removals, or a
  snapshot; full step-5 + oracle treatment); effects pool churns too fast to pay
  (exempt — its 16-slot sweep stays). Ceiling = the empty-slot tax only; per-live
  work (culling, dispatch) is real work. TRIGGER: Prof_TouchResponse/
  Prof_Peak_Frame showing pressure, OR the first object-vs-object demand,
  whichever first. Volence-ratified as jot-now-build-later (2026-07-10). — DESIGN-READY
- [tranche 8, 2026-07-10] **`.emp assert/diagnostics construct — FIRST demand data
  point** — rings' `assert.b d4, eq, #0` (DEBUG-fatal buffer-drop check) had to ship
  as a TRANSLITERATION (real asm skeleton + `dc.b` FSTRING data verbatim from the
  reference listing; kill row 16) because the real feature is a comptime
  format-string compiler over debugger.asm's console-token encoding ($E0 endl,
  $E8/$EA/$EC pal, arg descriptors) — debugger.asm-port-era machinery, not one call
  site's. Second demand ratifies designing it; the transliteration pattern covers
  singles until then. — **CLOSED 2026-07-12** by the diagnostics construct
  (`assert`/`raise_error`; sigil `diag-construct` branch, aeon retrofit `452c7c1`).
  The ratifying second demand: entity_window.asm's 11 assert sites (census: 30
  diagnostic sites corpus-wide). Grammar-level construct (parser → AST → desugar),
  FSTRING encoder in Rust; byte-IDENTICAL to debugger.asm's real `assert`/`RaiseError`
  macro (the CLI acceptance vector assembles the unmodified macro tower through the AS
  front-end and byte-diffs it). rings+core retrofit byte-neutral (kill row 16). The
  transliteration pattern is retired.
- [tranche 8, 2026-07-10] **`dc` link-expr cells** — the new `dc.b/w/l` proc-body
  statement (H8) is deliberately comptime-only (ints + strings); a link-resolved
  cell in dc position (`dc.l SomeLabel`) errors with a steering diagnostic. The
  extension is the D2.25 Value8/16/32 machinery already used by data items — build
  when a real consumer shows up (jump tables in code position are the likely
  demand). Z80 `dc` likewise designed-CPU-neutral (LE via stream_data) but
  unprobed — probe at the first Z80 code port. — OPEN (consumer-gated)
- [tranche 8, 2026-07-10] **`*` (current location) port-translation rule** — AS's
  `pea *(pc)` self-address idiom has no `.emp` spelling; the translation is a label
  on the instruction + `.label(pc)` (byte-identical d16=-2 encoding). Local-label
  displacement operands SHIPPED this tranche to make that expressible (parser
  DispInd continuation on the `Tok::Dot` arm). Goes in the D2.7/D2.19
  port-translation bucket (like `even`→`align 2`), not a language feature. — RECORDED
- [tranche 8, 2026-07-10] **typed view over a non-SST packed record — SECOND
  demand class** — the ring buffer's 6-byte entries (x.w, y.w, section_id.b,
  list_index.b) read via literal 0/2/4/5 displacements and hand-rolled ×6 index
  math (add/add/add chains at three sites). A `record`-over-raw-RAM view (the
  role-typed-SST cousin, vars-era neighborhood) would give named displacements +
  a sizeof-driven stride; the index-scale idiom (strength-reduced ×6) may want a
  comptime helper. Rings stays literal at transcribe; revisit when the construct
  gets its second consumer (entity_window's collected-window slots are the likely
  one). — OPEN (demand 1/2)
- [tranche 8, 2026-07-10] **hardcoded twin-guard counts CLOSED** — tranche 7's
  shared equ list still left per-test count literals; the twin's 18→24 growth broke
  6 targets. All counts now DERIVE from `test_support::engine_constant_equs().len()`
  (`twin_guards()`), composed per-module (30+N, 31+N, 34+N…). Future twin growth is
  list-edit-only, as originally intended. — CLOSED (tranche-8 back-prop)
- [tranche 8, 2026-07-10] **DrawRings culling literals** — `#336`/`#240` are
  `320+16`/`224+16` (screen + ring size) as comments; spelling them as derived
  constants needs screen-geometry names in the twin (+2 mirrors for 2 sites).
  Not worth the mirror tax today; becomes free after the constants.asm ownership
  flip (row 1). — RECORDED
- [tranche 8 step 5, 2026-07-10] **the GENERALIZED re-pin rule** — tranche 7's rule
  ("a region byte-change re-derives every SIGIL_EMP_* org between it and the next
  org boundary") is NECESSARY but not SUFFICIENT: the tranche-8 −4 shrink also hit
  mixed-map REGION BASES (sound_api/collision_lookup strings in the map fns),
  synthetic LABEL VMAs in port tests (Tile_Cache_GetCollision, Sound_DrainSfxRing,
  drain pointers), a probe's deliberately-wrong VMA (kept +4 from the NEW genuine),
  and a hardcoded BYTE-PIN ARRAY carrying a cross-region bsr.w displacement
  (tranche-5 mixed game_loop block). Rule as now practiced: a region size change
  re-derives EVERY harness pin whose value lies in [region_end, next org boundary)
  — orgs, map bases, label VMAs, byte arrays with displacement bytes, probe
  constants — all from listings. The org-$10000 boundary absorbs the slide
  (MDDBG__* verified unmoved from ROM bytes). Sweep grep: hex literals in the
  window over crates/*/tests + lib.rs, then let the strict suite name survivors. — RECORDED
- [tranche 8 step 5, 2026-07-10] **step-5 items deliberately NOT taken** —
  RingBuffer_Add's stack-round-trip ×6 (~24c, SPAWN-time cold path; fixing needs a
  wider clobber contract); RingBuffer_Remove's two remaining ×6 chains (COLLECT-time
  cold path; arbitrary-index remove can't roll); DrawRings (already
  rolling-pointer). Numbers recorded so future profiling has the baseline; the hot
  loop (RingCollision, per-frame × per-ring × per-player) got the rolling pointer. — RECORDED
- [post-t8, 2026-07-10] **bare-Bcc house rule RATIFIED + back-propagated** — Volence:
  conditional branches in .emp ports carry NO `.s`/`.w`; the assembler width-selects
  (the two-rung relaxation ladder). The rule was already PRACTICED in tranches 1-6
  (controllers/vdp_init/collision_lookup/sound_api are bare) but tranches 7-8
  (collision.emp 13, rings.emp 14) pinned widths citing the jbcc deferral — a
  drift the step-2 canonical text never named, now fixed there. Sweep result:
  ALL 27 stripped branches relax to their original widths (hand-written sizes
  were optimal) → byte-identical, no re-pin. Pinned exceptions, each commented
  in place: rings' assert-transliteration `beq.w` (macro-expansion parity, row
  16), aabb.emp's two `.s` (byte-locked to aabb.inc's explicit spellings —
  divergent relaxation between twins is the hazard). jbcc-the-MNEMONIC stays
  deferred — bare Bcc IS the idiom. — CLOSED (rule canonical in
  campaign-port-loop.md step 2)
- [tranche 9 step 1, 2026-07-10] **pc-rel target ADDEND shipped (demanded)** —
  `jmp .cc_table-4(pc,d0.w)`: the parser gives `.local` operand atoms binary
  continuations (`binary_continue` split out of `expr_bp`), eval folds the
  comptime addend (`label ± int` only; symbol stays link-time),
  `CodeOperand::PcRel{,Idx}` carries `addend: i64`, lowering emits `Sym ± n`
  through the existing `PcRelDisp8/16` fixup fold. Global-label `Sym±n(pc,…)`
  rides the same path (the `_` operand arm already parsed full exprs). — SHIPPED
- [tranche 9 step 1, 2026-07-10] **diagnostics ask: unexported-label hint** —
  `bra.w AnimateSprite.cc_delete` failed at LINK with "unresolved symbol
  `AnimateSprite.cc_delete`" when the label existed but lacked `export`. The
  fix (add `export .cc_delete:`) is not discoverable from the message. Ask: when
  an `Owner.label` reference misses AND `Owner` has a non-exported `.label`,
  say so and suggest the marker. One data point. — OPEN
- [tranche 9 step 2, 2026-07-10] **bare-Bcc lockstep procedure when the
  relaxation SHRINKS** — first occurrence: animate's five suboptimal hand
  widths (region 0x312→0x308). The .asm twin cannot go bare (the sigil AS
  front-end deliberately pins branch widths — "Aeon pins branch width, no
  relaxation"; bare Bcc is an .emp-only surface), so the lockstep move is:
  strip .emp widths → rebuild reference with asl (which DOES width-select
  bare spellings) → re-spell the twin's changed sites EXPLICITLY at the new
  optimal widths (commented) → verify identical hashes → full re-pin sweep.
  asl and the .emp relaxation were verified to agree on all five sites. — RECORDED
- [tranche 9 step 3, 2026-07-10] **AnimId/FrameId newtype demand point:
  AGAINST (interpreter-side)** — animate.emp does raw byte inc/dec/index
  arithmetic on anim/anim_frame; a newtype here would be cast ceremony with
  no misuse prevented inside the module. The real demand moment stays the
  MODULE BOUNDARY (player code ↔ anim tables), construct-walk #3 thread. — RECORDED
- [tranche 9 step 3, 2026-07-10] **interpreter duplication note** — AnimateSprite
  and AnimateSprite_PerFrame duplicate the control-code/event machinery (~90%
  same shape, different stream layout). A parameterized comptime-fn template
  could single-source it .emp-side, but the .asm twin cannot express the
  unification — divergent source SHAPES between twins raise the lockstep cost
  for every future edit. Deferred until the twin dies (Spec 5) or PerFrame
  gains a caller. — RECORDED (see also the D8 dead-export headline)
- [tranche 9 step 5, 2026-07-10] **step-5 items deliberately NOT taken (animate)** —
  (a) hot-path prologue (render_flags/status flip sync, ~56c/object/frame):
  alternatives cost the same 56c; behavior-load-bearing during frame holds.
  (b) `andi.w #$FF, d0` in both dispatchers looks dead but is LOAD-BEARING:
  it clears the high byte `add.w d0,d0` leaves when an anim id ≥ $80 — the
  read-through was verified, the instruction stays. (c) event-chain d1
  re-derivation (~16c/event) and the bra.w dispatch tables (24c/dispatch vs
  ~equal offset-table cost): control codes fire at script boundaries, not
  per frame — cold; not worth the upstream re-pin + lockstep tax. (d) the
  REAL candidate — deleting dead `AnimateSprite_PerFrame` (−404 bytes, zero
  callers) — is an engine-API scope call, headlined to Volence in the
  packet. LIVE-VERIFIED in oracle: anim-change path traced instruction-level
  (prev_anim write, DUR_DYNAMIC → d3 hold = 8, mapping_frame 7,
  piece count 5), Walk script cycling in the real game state, collision +
  rings (both slid −10) live under Sonic. — RECORDED
- [tranche 10 step 1, 2026-07-10] **imm-link + pinned-abs.w in one instruction
  (IMPLEMENTED, not deferred)** — core's `move.w #<link-imm16>, (<link-abs>).w`
  and `cmpi.w #<link-imm16>, (<link-abs>).w` (Init/Alloc free-stack pointer
  writes; ref `31FC 9EDE 9EDE` / `0C78 9E8E 9EDE`) demand a link immediate
  SOURCE plus a pinned-abs.w link DESTINATION — two independent fixups
  (Value16Be @2, Abs16Be @4). The imm-link path rejected any second symbolic
  operand ("fixups would collide" — over-broad; they're at different offsets).
  RULED a permanent capability (demanded-features law) and SHIPPED in step 1,
  not scaffolded: lower_m68k_imm_link admits ONE AbsSym{long} operand, second
  fixup at 2+imm_field_width. Relaxable Sym/SymOff still rejected (width
  selection genuinely conflicts). — IMPLEMENTED
- [tranche 10 step 1, 2026-07-10] **ImmWord16Be — the .w link-immediate range
  rule (corrected a shared-path regression)** — core's RAM-address immediates
  ($FFFF9EDE) forced the .w imm-link source fixup off Value16Be (unsigned
  [0,0xFFFF] — rejects the sign-extended address). The transcription pass first
  moved it to Abs16Be, but Abs16Be's EA-address window ([-0x8000,0x7FFF] U
  [0xFF8000,0xFFFFFF]) silently REJECTS an objroutine offset in [0x8000,0xFFFF]
  — a valid upper-bank tranche-6 store. Neither single-window kind is AS's
  actual word-immediate rule (high 16 bits all-0 OR all-1 = unsigned-value OR
  sign-extended-address union). Added FixupKind::ImmWord16Be (that union),
  routed the .w imm-link SOURCE to it; the abs.w DESTINATION stays Abs16Be (a
  real EA). Byte-neutral, strict 2086/0. Caught by adversarial review of the
  transcription agent's fixup-kind swap (not by a failing test — the regression
  was latent). — IMPLEMENTED (commit 80b6686)
- [tranche 10 step 3, 2026-07-11] **byte-lock friction: bare Bcc can't pin a width the twin forces** — `bne.w RunObjects_Frozen` sits within `.s` range (disp 0x7E=126) yet MUST stay explicit `.w` to match the AS twin's `bne.w`. Bare Bcc relaxes to the shortest reaching width, so it over-relaxed to `.s` and the byte gate caught it (candidate 446 vs ref 448). Recurring pattern (animate had the bra.w table; every debug/macro-expansion byte-lock). ASK: a force-width idiom (e.g. `bne.w!` / a `pin_width` attribute) OR a twin-parity lint so these byte-locks are DECLARATIVE, not landmines caught only by the gate. Until then the rule is "explicit width + a `// byte-lock:` comment." No back-prop code change (prior files' bare Bccs all happen to relax to their twin's width — latent fragility, not a current bug). — RECORDED (ask)
- [tranche 10 step 3, 2026-07-11] **`org $10000` shields downstream from engine-block shrinks** — core's −4 shrink did NOT move EndOfRom or any object-bank/data region: the object bank is re-anchored at `org $10000` ~42KB past the engine block's end, so an engine-internal shrink is absorbed as extra pad before the bank. PROCESS NOTE for future tranches: an engine-block-internal shrink only re-pins engine-block-downstream regions (up to the next `org`), NOT the object-bank/data regions. Don't budget re-pins for those (the ASSEMBLED_LEN "−4 both shapes" prediction was wrong for exactly this reason). — RECORDED
- [tranche 10 step 3, 2026-07-11] **repin can't track inline target BYTES in mixed-test slices** — a byte-array slice that hard-codes an abs.w/pc-rel TARGET's low word (test_solid's `jmp (Draw_Sprite).w`, game_loop's `bsr.w Sound_DrainSfxRing` disp) slides when the target moves, but it's a `[u8]` literal, not a pin. Mitigation applied: splice `pins::DRAW_SPRITE.<shape>` low word into the array instead of a hex literal. GENERALIZATION candidate: a `pin_lo16(pins::X)` test helper (or a repin mode that rewrites known-target bytes in slices). — RECORDED
- [tranche 10 step 5, 2026-07-11] **RunObjects profiled (numbers, not vibes)** — oracle profiler, live in-level with 3 active objects (Player + 2 TestSolid): RunObjects = **11,841 cycles = 9.3% of the 128k NTSC frame budget**, dominated by fixed iteration over all 66 pool slots (2 player + 40 dynamic + 8 system + 16 effect), ~63 EMPTY in this scene. The dispatch loops ($0028B6 ×3 = 9,677 cyc) are the bulk. **Two step-5 candidates, BOTH NOT TAKEN:** (a) hoist the `moveq #OBJ_CODE_BANK; swap d0` bank-prefix build out of the per-slot loop (currently rebuilt every iteration incl. empties, ~8cyc×empty ≈ 200cyc/frame in a light scene) — but it ADDS ~8 bytes (one-time setup before each of 2 loops) → reverses part of the shrink → another downstream re-pin, for 0.15% budget. Bad trade. (b) the REAL lever — an occupancy/active-list so RunObjects skips empty slots instead of iterating all 66 — is an ENGINE-ARCHITECTURE change (the pool-iteration contract), behavior-affecting, Volence's call (same class as the tranche-9 PerFrame deletion). Headlined to Volence in the packet, NOT taken unilaterally. DeleteObject's 20× unrolled $50-byte clear: deletion is not per-frame-hot (didn't surface in the profile), the unroll is a fine size/speed choice — recorded, not taken. — RECORDED (numbers + not-taken)
- [tranche 10 step 5, 2026-07-11] **dplc dedup (carried from step-1 H5)** — Perform_DPLC / Perform_DPLC_Deferrable are near-identical (only the QueueDMA_Important vs _Deferrable target differs). A comptime-fn template (`perform_dplc(queue: ProcRef)`) would single-source it .emp-side, but the AS twin can't express the unification → divergent twin SHAPES raise the lockstep cost for every future edit (the animate PerFrame-interpreter reasoning). Deferred until the twin dies (Spec 5). — RECORDED (deferred)
- [table design review, 2026-07-11] **packed pointer-composite CELLS are the gate for typing
  zoneanim/dbglist rows under the `table` construct** — `dc.l (dur&$FF)<<24|artaddr` /
  `frame<<24|obj` / `levartptrs`' `plc<<24|art` need a link-time `imm<<24 | Abs32` composite
  fixup, sibling of t10's imm-link-with-pinned-abs and the win-tab's `(Sym & mask) | base`
  Value16Le. Correctly scoped OUT of `table` (its own cell-level item, feeds the construct);
  flagged here so a future zoneanim/dbglist/MLLB port doesn't stall surprised — the collection
  framing applies immediately, the record INTERIOR waits on this fixup kind. Design note:
  specs/2026-07-11-counted-sparse-collection-design.md §6 boundary 1. — OPEN (consumer-gated)
- [out-clause, 2026-07-11] **auto-inc/dec write-analysis gap (surfaced by out-unwritten)** — the register-write detection behind BOTH `[proc.clobber-undeclared]` and the new `[proc.out-unwritten]` only counts a BARE register destination (`move x, a4`); it MISSES a register modified via post-increment `(An)+` or pre-decrement `-(An)` (`move x, (a4)+` advances a4 but isn't detected). Two consequences: (1) the clobber lint silently under-reports — a proc that scribbles a4 via `(a4)+` without declaring it gets no warning (~44 non-sp auto-inc/dec sites across the engine corpus, mostly on already-declared scratch pointers a0/a1/a2, but some on in-out output pointers); (2) `out(a4)` for a genuine in-out pointer output (DrawRings' advanced SAT pointer) false-positives `[proc.out-unwritten]`, so it's left undeclared with a comment. FIX (its own focused pass): count `(An)+`/`-(An)` operands (source OR dest) as writing `An` in the write-detection; then audit the newly-surfaced (correct) clobber warnings across the corpus and declare them. Enables pointer-output `out(a4)`-style contracts. — RECORDED (follow-up)
- [C2 let-binding, 2026-07-11] **binding-consistency lint is the S2-D6/D7 dataflow demand** — `let a2: *Sst` is an ASSERTION, not a verification (param-identical trust: nothing checks a2 actually holds an Sst pointer). The obvious lint — a WRITE to a bound register between the `let` and a later typed use (`let a2: *Sst` … `move.w #0, a2` … `x_pos(a2)`) is a stale/lying binding — is a DATAFLOW pass (needs flow-sensitivity, which the lexical binding deliberately is not, v1). Recorded as the demand row for S2-D6/D7's dataflow pass; NOT gated on here (params shipped on the same trust). The same pass also homes the auto-inc/dec write-analysis gap (out-clause row above) and flow-sensitive typing (`let` narrowing after a branch). — RECORDED (deferred, S2-D6/D7)
- [C2 let-binding, 2026-07-11] **rings.emp ring-buffer half is paired to the `table`/`*RingEntry` record view, NOT `let`** — TouchResponse retrofit shipped (13 `Sst.field(a2/a3)` → bare `field(aN)`, byte-identical both shapes). The ring-buffer half of rings.emp was deliberately NOT retrofitted: its 6-byte entries want a record-OVER-raw-RAM view (a `*RingEntry` struct-over-buffer), which `let aN: *RingEntry` will CONSUME once the counted-sparse-collection/`table` construct lands a RingEntry struct (see counted-sparse-collection design + the packed-composite-cell ledger row). `let` types a register against an EXISTING struct; it does not itself define the record-over-RAM layout. Don't front-run — the pairing is: land RingEntry (table), THEN `let aN: *RingEntry` retrofits the ring loop. — RECORDED (consumer-gated pairing)
- [table construct, 2026-07-11] **`table` SHIPPED (Plan 7 T2-d) — deferred boundaries ledgered** — the counted/sentinel/sparse collection construct landed on the `table-construct` branches (sigil + aeon), front-end only (zero sigil-ir/sigil-link change), byte-neutral. Acceptance: sfx_bank.emp retrofit byte-identical BOTH shapes (196→~90 lines, cross-seam Sfx_NN win-tab reads resolve, guard count stays 1) + a record-list PLC vector byte-diffed against the AS `plrlistheader`/`plreq` macros. Spec draft: specs/2026-07-11-counted-sparse-collection-spec-4.9-draft.md (§4.9 + D2.36, awaiting Volence's empyrean-cadence paste). **Deferred, each consumer-gated:**
  - **auto-labeled keyed record rows** (index mode over typed records without explicit part labels — cell points at an auto `Name.$NN` row label): DEFERRED, no demand instance needs it; v1 is explicit-labels-only. Design decision 6. — OPEN (consumer-gated)
  - **interior header-end labels (`NamePlc`)**: LEDGERED not built — the AS `plrlistheader` macro also defines an interior label after the count word; the construct doesn't. Build a `plc_label:`/`header_end:` knob only if a classic-Sonic PLC-list port finds real references to them. Design decision 7. — OPEN (consumer-gated)
  - **keyed-WITHOUT-cell dense record-list** (the S2 zone-ordered "N slots per key" tables, 568 entries / 28 decls / `!org` math): v1 validates ascending/dup/range keys and emits the rows contiguously, but the exhaustive-dense re-flow by key (row type `[u16; N]` per zone, missing-zone-row → compile error) is only wired for the INDEX (cell) path. A keyed record-list emits rows in decl order; the per-key dense slotting is untested until a classic-Sonic zone-table port needs it. — OPEN (consumer-gated)
  - **enum / `offsets`-name key domains**: v1's `key:` is an inclusive integer range (`lo..=hi`) only; the design's `key: ZoneId` (enum) / `key: SomeOffsets` (ordinal set) domains are a later increment (the exhaustive zone-table instance is the first consumer). — OPEN (consumer-gated)
  - **cross-module `pub table` resolve integration**: the table's base label + part labels are real link symbols (define_label'd, linker-global — the sfx cross-seam .asm reads prove it), but `pub table` export through resolve/mod.rs's symbol-collection pass (rename hygiene, cross-module duplicate-label detection for `.emp`→`.emp` refs) is unexercised by the acceptance (sfx is header-less + read cross-seam by .asm). Wire + test when the first `.emp` module references another module's `pub table` by bareword. — OPEN (consumer-gated)
- [table step-6 sweep, 2026-07-11] **table adoption sweep across the converted corpus — sfx_bank was the only clean win** (ran per the ratified step-6 rule when `table` shipped). Findings: (a) **mt_bank.emp — NOT retrofitted.** Its SongTable/SongPatchTable are DENSE (`[*u8; SONG_COUNT]`, no holes — table's hole-fill gain is zero), only 1-3 rows (already concise), and it needs THREE table features it lacks: DEBUG-CONDITIONAL rows (Song_DrumTest/Song_HCZ2 only in debug), a PARALLEL second cell table (SongPatchTable over the same rows), and a NON-1:1 cell (DrumTest's patch cell → MovingTrucks_Patches, not its own row's label — table cells point at the row's first label only). Retrofitting would mean BUILDING three features to express something already concise = net-negative. Ledgered as the demand signal IF a future dense-conditional-multi-table file (classic-Sonic zone/level tables) makes those three features worth it: `table` conditional rows, parallel/multi cell tables per block, arbitrary-label cell targets. (b) **dac_samples.emp — NOT a candidate.** Its DAC descriptor table is `.asm`/Z80-side (dac_sample_tab.asm reads the SND_* equs); the .emp holds only the sample blobs + equs (like sfx's win-tab, out of scope per R3). (c) **act_descriptor.emp — marginal.** Its 9-entry Sec record list is dense and already uses a validating `ojz_sec()` constructor; table's record-list mode could express it but gains little. Low-priority; revisit if a second act makes the pattern recur. — RECORDED (sweep ran, retrofit-or-ledger → ledgered; sfx_bank remains the sole retrofit)
- [tranche 11 sprites, 2026-07-11] **`Sym ± const` absolute-address operands SHIPPED (demanded feature)** — sprites.emp's `btst #0, (Sprite_Cycle_Counter+1).w` (odd byte of a word RAM cell) needs a bare symbol + constant byte offset in an EA. The bare-label idiom lowered fine but `Sym+1` comptime-folded the link symbol → "unknown name". Fixed in `eval/asm.rs map_plain`: `sym ± const` (either order for `+`, sym-left for `-`) → `CodeOperand::SymOff`, riding the existing `RelaxAbsSym` width-rule seam (same as `Item.field`). 3 byte-level tests. Step-6 corpus sweep: ZERO retrofit sites (no prior `.emp` has a `Label+N` memory operand) — sprites is the sole user so far. — SHIPPED (sweep clean)
- [tranche 11 sprites, 2026-07-11] **Emit_ObjectPieces four-variant unification — BUILD ATTEMPTED, BLOCKED by a language gap (Volence-approved; gap discovered mid-build)** — the four inline flip-variant piece loops (unflipped/xflip/yflip/xyflip, ~160 lines) share a SAT-write skeleton differing only in Y-transform, X-transform, tile `eori` mask, and size-byte source. Attempted `comptime fn emit_piece_loop(xflip, yflip) -> Code` building the loop imperatively (`comptime var body`, `body ++ asm{...}` per conditional segment). **BLOCKED: labels do NOT resolve across `++`-concatenated `asm{}` fragments** — each `asm{}` block is its own hygiene scope, so the loop-back `.piece_loop:` (defined in the reads fragment) is unresolved from `dbeq d4, .piece_loop` (in the tail fragment): `unresolved symbol .piece_loop for fixup`. A runtime loop whose body varies conditionally CANNOT be assembled from concatenated fragments, because the loop-back label must span them. Relocating the four full single-block variants into the fn gives NO dedup (just moves code). **THE LANGUAGE ASK:** either (a) `++` preserves labels across fragments within one comptime-fn instantiation (per-instantiation hygiene scope, not per-`asm{}`-block), OR (b) `asm{}` supports splicing a Code-fragment value (`{code_val}` — the `CodeItem::Inline` gap, Plan-4-unreachable) so the whole loop stays ONE block with the conditional middles spliced in. Either unblocks this (and any conditional-body loop template). Reverted to the byte-exact four-inline-variant step-2 state. — **CLOSED 2026-07-11** by the `asm{}` Code-splice feature (`{expr}` at statement position, mini-spec ratified): the emit_piece_loop skeleton-with-holes now lives in ONE block (label + dbeq) with label-free `{term()}` splice holes; retrofit is byte-IDENTICAL to the four inline variants both shapes. The chosen fix was (b) Code-fragment splice into `asm{}`, NOT (a) cross-fragment label scope.
- [asm code-splice, 2026-07-11] **cross-fragment per-instantiation label scope — the ALTERNATIVE fix, LEDGERED at zero demand** — the `asm{}` Code-splice feature (`{expr}` at statement position) chose fix (b): a spliced fragment inlines into ONE skeleton block, so the skeleton owns its labels and the holes are label-free. The alternative (a) — make `++`-concatenated fragments within one comptime-fn instantiation share a label scope, so a fragment could DEFINE or REFERENCE a label the skeleton branches into — stays UNBUILT. Hygiene is deliberately unchanged (per-block scope). Its ratifying case would be a conditional fragment that must OWN a label the skeleton branches into (not expressible with skeleton-owns-labels). No such demand today; splice covers every current need. — RECORDED (zero demand; escalation path)
- [asm code-splice, 2026-07-11] **`Data` value splice into `asm{}` — LEDGERED at zero demand** — a `{expr}` splice whose expr yields `Data` (not `Code`) is a STEERING error today ("data belongs in `dc`/`bytes()`; a Data splice is unbuilt"). Splicing raw Data bytes INTO a code stream (the Plan-4 `CodeItem::Inline`-for-Data direction — e.g. inlining a `dc.b` blob mid-proc) has no demand instance. Build it only when a real port needs data interleaved with code that `dc` inside the block can't already express. — RECORDED (zero demand)
- [oracle profiler, 2026-07-11] **profiler measurement-caching bug (stale after ROM reload)** — during the A1 camera-bias oracle verification, `emulator_get_profiler_frames` returned BYTE-IDENTICAL cycle counts across two separate measurement runs of DIFFERENT ROMs (master vs A1) — e.g. `VSync_Wait` = 69805 cyc in both, statistically impossible for a live re-measurement (cycle counts jitter frame-to-frame). Even after `set_profiler(false)`+`set_profiler(true)` and a fresh ROM reload, the frame buffer appears to serve pre-reload data. Consequence: no clean numeric profiler delta could be extracted; the A1 win had to be argued structurally. FIX (oracle, its own task): flush/reset the profiler's accumulation buffer on ROM reload (or on set_profiler(true)) so post-reload reads reflect the new ROM. — FIXED 2026-07-12 (oracle `linux-port`, +56 lines across `main_gui.cpp`/`ControlSocket.{h,cpp}`, pending merge): the accumulation buffers (`profilerHistory`, `profilerFrameCount`, `profilerPendingEvents`, and the CPU-side `ProfileRingBuffer`) are now flushed at BOTH boundaries — (a) ROM-reload completion (in the main-thread drain, beside the device-pointer refresh), and (b) profiler enable. Boundary (b) is a main-thread-honored RESET-REQUEST FLAG (`Context::profilerResetRequest`), not enable-edge sampling: the two lessons that shaped the fix were (1) all clearing stays on the main thread — `set_profiler` only SETS the flag, the frame loop clears — because mutating the history off-thread reintroduces the UAF/race class the reload path guards against (ProfileTypes/ControlSocket device-resolution note); and (2) a flag beats edge-detection because a rapid `set_profiler(false)`→`(true)` pair the main loop never observes as `false` would miss a false→true edge (the exact C3 failure caught during verification). Verified against all four acceptance criteria (same-ROM jitter live; A→B different-build no carryover, history empty immediately post-reload; frames_recorded restarts from 0; 6× reload soak no crash) + determinism gate 120/120 (no regression). CLOSE on merge.
- [sprites A1, 2026-07-11] **A1 camera-bias fold — clean stress-scene re-measure owed** — the camera-bias fold's cycle win (−16 cyc/piece, ~1k cyc/frame ceiling at the 80-sprite `MAX_VDP_SPRITES` SAT cap, break-even ~4 pieces) was verified STRUCTURALLY (removed instructions) + behaviour-identical (SAT + framebuffer byte-identical, pixel-locked). It was NOT cleanly profiled because the only available scene (OJZScroll) is VSync-bound (54% idle) with 3 sprites, AND the profiler caching bug (above) blocked a delta. Re-measure under a near-full-SAT stress scene (the object-test state, many pieces on screen) for a clean numeric win once the profiler bug is fixed. — RECORDED (verification follow-up); UNBLOCKED 2026-07-12 by the profiler caching fix above (re-measure now trustworthy).
- [sprites C1, 2026-07-11] **InsertSpriteMasks leader-sprite guarantee — ACCEPTED-limitation, ledgered (Volence)** — the VDP X=0 sprite-mask FIRST-SPRITE-ON-LINE exemption: masking takes effect only when an earlier-linked (higher-priority) sprite already touches the masked scanline. `InsertSpriteMasks` inserts at a band boundary, so over an EMPTY high band the mask is first-on-line and silently fails to hide. Volence's ruling: ACCEPT as a known limitation (documented at the `InsertSpriteMasks` site), and ledger the fix. FIX (consumer-gated): guarantee a leading (higher-priority, earlier-linked) sprite on the masked scanlines before the mask — e.g. `InsertSpriteMasks` emits a transparent 1×1 leader on each masked line, or a documented scene contract that any masked band always has content above it. Build it when a real scene needs masking over a potentially-empty band (no demand instance today — SpriteMask_Y defaults 0 = disabled). — RECORDED (consumer-gated; accepted limitation)
- [F3 controllers, 2026-07-11] **controllers.emp P1/P2 latch helper — LOOKED AT, DECLINED (name-the-idiom gate)** — Read_Controllers repeats a 6-instr latch block per player (`lea HW_PORT_N; jbsr .read_pad; move Ctrl_N_Held,d1; move d0,Ctrl_N_Held; eor; and; or d1,Ctrl_N_Accum`), a clean 2× structural clone. A `latch_pad(port, held, accum)` comptime-fn would name it, BUT the template body must `jbsr .read_pad` (a proc-LOCAL label) — hygienic template labels can't resolve an enclosing-proc local (the same cross-fragment label-scope gap that blocked emit_piece_loop). Making `.read_pad` a module proc = a bigger structural change than the 66-line file warrants. Declined per the 4(b) taste gate; would be unblocked by the fragment-label-resolution ask (emit_piece_loop row). — RECORDED (declined; unblocks with fragment-label scope)
- [oracle harness, 2026-07-11] **object-test state fights static memory setup (R-A1 / Bug-2 live-repro blocked)** — verifying the R-A1 ring-cull boundary and the Bug-2 push-flicker needs a controlled static scene (a ring pinned at a screen edge; a grounded player flush against a terrain wall). The object-test state actively RE-DRIVES the ring buffer + camera every frame and defaults the player into debug-fly (`debug_flag=$FF`, forced PSTATE_AIR) hovering over bottomless pits — so a `write_memory` setup is overwritten within one frame and the player can't be parked grounded next to a wall. R-A1 was proven by cull-math derivation + byte gate + a SAT-emit read; Bug-1 was proven by a deterministic 1-frame camera-Y discrimination test (which survives because physics preserves the y_vel sign). FIX/owed: a physics-freeze / scene-pin debug hook, OR a normal playable level, for pixel-boundary (R-A1) and grounded-wall-push (Bug-2) live confirmation. — RECORDED (verification tooling)
- [diag construct, 2026-07-12] **`Console.*` / `KDebug.*` construct — DEMAND 0** — the interactive-debugger surface (live console print, KMod breakpoints) beyond `assert`/`raise_error`. ZERO corpus sites (census: 0 Console/KDebug). The grammar reserves NOTHING for it. Build only if a real `.emp`-ported file wants live console output. — RECORDED (demand 0)
- [diag construct, 2026-07-12] **`consoleprogram` (`raise_error` two-arg form) — DEMAND 0** — the second `raise_error` argument that swaps in a full console program instead of a fatal message. ZERO sites; today `raise_error` with a second arg is a loud steering error (spec §5). Un-defer when a game-side site needs the console-program form. — RECORDED (demand 0)
- [diag construct, 2026-07-12] **memory-operand assert/raise_error arg push — DEMAND 0** — `assert` `src` must be a register in v1 (matches debugger.asm's own parenthesised-operand limitation, AS error #1300); a memory operand is a loud "move to a register first" error. Corpus practice already loads to a register (rings' `move.b Ring_Add_Dropped, d4`). Build the memory-operand arg-push path only if a port hits a genuine memory-comparand assert that can't restructure. — RECORDED (demand 0)
- [diag construct, 2026-07-12] **comparison-operator assert sugar — POST-SPEC-5 TASTE** — `assert.w d1 <u #MAX` sugar over the cc-mnemonic form (`assert.w d1, lo, #MAX`). The cc form is the porting workhorse (AS-parity, copy-paste); the operator sugar can layer later without breaking it. Deferred by decision (spec §9) — a readability bikeshed, not a gap. — RECORDED (post-Spec-5 taste)
- [diag construct, 2026-07-12] **operand-spelling capture is SCAFFOLDING-ERA** — the assert auto-message embeds the source operand SPELLING verbatim (§4.4), captured by slicing the source span at parse (`Parser` holds an `Rc<str>` of the source; slice-at-eval was infeasible — `Evaluator`/`lower_module` carry no source). Byte-exact by construction for ALL operand forms (hex/bin literals survive: `#$8000` stays `#$8000`). The hazard this guards — a spelling that diverges from the AS twin's message bytes — only bites DUAL-BUILD sites (a twin exists to diverge from); it is scaffolding-era like the twins themselves, and the restriction lifts at Spec 5 with the message format. — RECORDED (scaffolding-era; lifts at Spec 5)
- [diag retrofit sweep, 2026-07-12] **grown retrofit follow-ons — DeleteObject range + animate underflow asserts (NOT yet ported)** — the retro-review surfaced two more assert-worthy sites beyond rings+core: DeleteObject's out-of-range slot check, and animate's cyclic `AF_CHANGE` + `AF_BACK` underflow. These are follow-on retrofits (or new asserts) for a LATER step-6 sweep when those files port — explicitly OUT of the diagnostics-construct build's byte-neutral acceptance (rings+core only). Convert them with the construct when their files reach the port queue. — RECORDED (follow-on sweep sites)
