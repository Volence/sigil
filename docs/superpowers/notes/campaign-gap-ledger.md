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
