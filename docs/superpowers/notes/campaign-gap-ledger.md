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
