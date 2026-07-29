# The port loop (canonical — Volence-ratified 2026-07-10, supersedes all prior loop descriptions)

**THE SHAPE (Volence-clarified 2026-07-16 — "steps 2-6" / "3-6" phrasing
keeps blurring this; the numbers are NOT a linear sequence):**

    0 → 1 → 2 → (3 → 4 → 5)* → 6 → merge

Steps 0/1/2 run ONCE per file. **The loop is 3→4→5**, and **DRY means a
FULL pass comes up empty at ALL THREE steps** (Volence, 2026-07-16):
step 3 finds nothing, step 4 adopts/builds/asks/deletes nothing, step 5
takes nothing. An empty step-3 retrospect ALONE is not dry — the steps
uncover each other's items (a step-5 optimization can create a
reads-wrong or a construct opportunity; a step-4 helper can open a
hoist), so the exit requires one whole 3→4→5 circuit with zero findings
end to end. **Step 6 runs ONCE, after that dry circuit — never inside
the loop.** Writing "steps 3-6" as if 6 were a loop member is the
recurring error this box exists to stop.

**CHANGELOG (re-read this doc at each step boundary — it changes
mid-campaign, and a ruling ratified after your last read still binds
you):**
- 2026-07-29 (2nd): NINE already-practiced rules codified (Volence-ratified,
  overseer-dispatched — the game-side conversion arc t24→t34 ran them as
  precedent; text closes the drift channel the abs-EA catch exposed). Into
  the step-2 checklist as items 7-10: module-header requirements (C3's audit
  surface), the symbol-resolution ladder (`import` → bare link → `extern()`
  const+guard → `-D`; `extern proc` prohibited bar a die-at-port boundary
  decl), honest-contract derivation (over-claim clobbers is SAFE; never an
  unverifiable `preserves`), `as`-bless placement (on the producer). Into the
  loop text: oracle A/B for byte-gate-blind changes (step 5); the
  standing-patterns registry + the wave-ripple checklist (new subsections);
  panel-activation rules (C3-on-hardware-claims / C1-flagged-call, dry-panel);
  forced-spelling site comments generalized (step 2). Noticing clause renumbered
  7→11.
- 2026-07-16 (2nd): DRY refined — the exit is a FULL 3→4→5 pass empty
  at ALL THREE steps, not an empty step-3 alone (the steps uncover each
  other's items).
- 2026-07-16: THE SHAPE box added (loop = 3→4→5 until dry; 6 = one
  final post-loop pass; 0/1/2 once per file).
- 2026-07-15 (3rd): loop self-extension audit — step-0 TRIP-CHECK (kill
  conditions this port trips, not just file-keyed rows) + probe
  BINDING-CLASS rule; step-1 GATE-ARTIFACT discipline (every gate names
  its test/commit) + PROOF-MECHANISM feed-forward (ownership-flip class
  requires its two-module link test per flip); noticing clauses added to
  the 3(a)/3(b)/5 interrogations (all three now self-extending).
- 2026-07-15 (2nd): step-2 CHECKLIST formalized (was the only judgment
  step with no enforced list) + the NOTICING clause (step 2 is
  self-extending per file) + the FEED-FORWARD rule (a shipped feature
  with a spelling implication adds its step-2 line same-change).
- 2026-07-15: macro-port rule (a donor macro's .emp counterpart is an
  INTERFACE REDESIGN, not a transliteration — named block before Step 4;
  hooks in Steps 1 and 4; ratifying it obliges a one-time retroactive
  enumeration sweep of already-ported macro counterparts, ledgered).
- 2026-07-12: register-contract convention (`clobbers()` = exhaustive
  license, before Step 4); step-6 pattern-enumeration amendment;
  packet step-5 section = filled per-line checklist (below); step-4
  kill-row same-commit rule made explicit; step-0 ledger sweep for
  file-implicating hazards.
- 2026-07-11: step-3(a)/3(b)/5 interrogation checklists; step 4
  Construct pass; step 6 Corpus sweep.

Per file/tranche. The byte gate is a STEP-1 artifact (the transcribe
verifier), not a permanent style cage — after step 1 proves identity,
later steps may change bytes, paying the lockstep + re-pin tax each time.

**After step 1 the goal is BETTER, not SAME** (Volence, 2026-07-11).
Byte-exactness to the original is proven at step 1 and never owed again.
The continuous gate is only `.emp == AS twin` — two forms of the same
file agreeing — and it is satisfied by EDITING BOTH SIDES in lockstep,
never by freezing the .emp to the twin's stale spelling. "The twin has
X", "it wouldn't be byte-exact", and "re-pin is work" are never reasons
to keep old spelling or skip an improvement: the re-pin tax is the
routine, budgeted cost of steps 2–5. Declining an improvement is a
step-5 decision logged in the packet ("not taken, because X") — never a
silent default.

**Step 0 — Recon + design** (when the file carries hazards): read the real
tree, extract per-shape addresses, settle design questions in a written
note BEFORE code. Trivial files skip it. **Hazard sweep (added
2026-07-12, from the t12 blind-run miss): grep the campaign gap-ledger
for this file's name and its proc symbols** — audit/spec findings that
implicate a file are ledgered against it (that's the handoff channel;
an in-flight hazard you didn't sweep for is a step-0 failure, not bad
luck). **Trip-check (added 2026-07-15, from the t15 row-22 miss — the
SECTION_SIZE due-bill was const-keyed, not file-keyed, and the sweep
walked past it):** the sweep ALSO reads kill-list/ledger KILL CONDITIONS
this port could TRIP — consts/symbols the file consumes, "Nth consumer"
clauses that this port satisfies, and at-next-touch rows naming any file
the tranche touches. Corollary for row authors: at-next-touch rows must
NAME their files explicitly, or the sweep can't surface them.

  **Probe binding-class rule (2026-07-15; the lesson cost a wrong step-0
  diagnosis in the sst-usability batch and was never written down):** a
  confirming probe must replicate the real site's BINDING CLASS —
  comptime vs link-time, gated vs ungated, extern vs local — not just
  its expression shape. A probe that "confirms" a spelling against a
  comptime base says nothing about the link-time sites it claims to
  clear. Design notes state each probe's binding class next to its
  conclusion.

**Step 1 — Transcribe**: the 1-1 faithful port. Same mnemonics, same
explicit widths, comments carried. Gate + region pin + byte gates both
shapes + mixed-build acceptance + negative probes + gate-off neutrality.
Language features the file DEMANDS ship here (the demanded-features law).
Byte-exactness is proven HERE and only owed here. When the demanded
feature is the .emp counterpart of a donor MACRO or AS function, its
interface is designed under the macro-port rule (below) BEFORE it is
built — the byte gate binds the counterpart's emitted bytes, never its
signature.

  **Gate-artifact discipline (2026-07-15, from the t15 mixed-build row
  that described a run that never happened + the acceptance bar's silent
  drift across tranches):** the packet's step-1 section is a FILLED gate
  list where every gate names its ARTIFACT — the test fn or commit that
  proves it ("mixed acceptance: `two_module_ownership_flip_*`", "negative
  probe: <test>", "gate-off: <CRCs>"). A gate row without an artifact is
  a claim, not a gate; checklist rows describe what was EXECUTED, never
  what the plan intended.

  **Negative probes need a POSITIVE CONTROL in the same file (ratified
  2026-07-24, t24 — the class was invisible to every gate the campaign
  runs):** an `assert_ne!`-shaped probe ("a doctored X must NOT match the
  reference") only means something if the UNDOCTORED compile through the SAME
  path DOES match. Where that claim lives in prose, the probe's fixtures rot
  silently: `tranche3_negative_probes.rs` spent several re-pins comparing a
  0x24-byte section against a 0x1A8-byte reference slice through hardcoded
  addresses, so both its probes were trivially true and could not fail. Every
  negative-probe file therefore carries an executable positive control
  (`*_undoctored_compile_equals_the_reference_window`), and its fixtures are
  PINS-DERIVED so a re-pin cannot re-stale them. Verify a new probe BOTH ways:
  it must fail when doctored and pass when not.

  **Proof-mechanism feed-forward (2026-07-15, same shape as step 2's
  feed-forward rule):** when a port creates a NEW seam-configuration
  class and builds its proof, that proof becomes REQUIRED for every
  future instance of the class — named here so it binds: the
  symbol-ownership FLIP class (an already-ported .emp module's calls
  re-resolving to a newly-ported .emp owner) requires a persisted
  two-module link test per flip (t15 section/entity_window is the
  template; plane_buffer's port flips Draw_TileColumn for section.emp
  and owes the next one).

**Step 2 — Modernize**: convert to the complete house format. ALL
control flow goes new-style — `bra.w`/`bra.s`/`jmp` → `jbra`,
`bsr.w`/`bsr.s`/`jsr` → `jbsr` (`jmp`/`jsr` stay ONLY for computed
targets), conditional branches go BARE (no `.s`/`.w` — the assembler
width-selects over the relaxation ladder; ratified 2026-07-10). The ONLY
width exceptions are STRUCTURAL, each with a site comment naming which:
(1) transliteration blocks pinned to a macro expansion, (2)
stride-locked jump-table slots (load-bearing `bra.w` tables). "The AS
twin has an explicit width" is NEVER an exception (clarified 2026-07-11;
the core.emp `bne.w RunObjects_Frozen` pin was a misapplication of the
old wording) — the twin is ours: when a bare Bcc relaxes, shrink the
twin in lockstep and re-pin, same as any step-2 byte change. Also:
bare-symbol width-rule spellings, new-style idioms. Bytes MAY change;
constraint is BEHAVIOR-IDENTICAL (spelling/idiom/layout/dead padding —
no logic change). AS twin edits in lockstep, pins re-derived, gates
re-green. No emulator time needed at this step.

  **Brace-indent rule** (Volence, 2026-07-11; style, not
  assembler-enforced): every `{` block body indents ONE level — `if`/
  `else` blocks (`if DEBUG == 1 {`), construct bodies, any braces. Labels
  inside a block keep their usual shallower offset relative to that
  block's instructions. The eye must see membership without hunting for
  the `}`. Formatting-only (bytes unaffected); existing files re-indent
  at their next touch, not as a dedicated wave (a future `sigil fmt`
  would mechanize this — ledgered, zero tooling demand yet).

  **The step-2 checklist (Volence-ratified 2026-07-15 — step 2 previously
  had NO enforced list, making it the easiest step to declare done on the
  loudest item; the packet's step-2 section is now the FILLED checklist,
  same enforcement as steps 3(a)/3(b)/5):**

  **META (2026-07-29, Volence's abs-EA catch): a step-2 MISS traced to an
  UNSTATED (precedent-only) convention makes CODIFYING that convention a
  mandatory close-packet duty. The byte gate is blind to byte-identical
  spelling variants, so a convention that lives only as precedent in engine
  files drifts silently in new files until it is TEXT here — the exact failure
  behind the abs-EA catch (the bare-RAM-EA form obeyed by every engine file but
  never written down, so three game-side files drifted for three tranches). The
  abs-EA idiom (item 5), the contract CPU-split (item 5 reglist), and the
  mem-to-mem comment string (item 2) were all promoted from precedent to TEXT by
  this rule after the 2026-07-29 convention-drift audit.**
  1. Branch conversions (bare Bcc, `jbra`/`jbsr`) INCLUDING the
     region-shrink/re-pin/twin-lockstep wave they trigger — the packet
     names the byte delta and where downstream absorbs it (org shield vs
     re-pin propagation). Checked, not assumed. SHAPE-DIVERGENT
     relaxations (a span that relaxes in one shape only — typically
     crossing a debug-only assert blob or a comptime arm) ride TWIN
     `ifdef` widths: the .emp stays bare, the twin carries the per-shape
     spelling with its site comment (t19 bg_anim · t21 vblank sound-off ·
     t22 s4lz; overseer-ratified 2026-07-24 from the t22 noticing,
     Volence-ratified 2026-07-29).
  2. Every structural width-pin carries its site comment naming the
     exception class — an UNCOMMENTED kept width is a miss even when
     keeping it is correct (retro-check 2026-07-15: `aabb.emp:62 bpl.s`
     was the corpus's single uncommented pin; everything else conformed).
     A mem-to-mem `move.w (X).w, (Y).w` (TWO symbolic operands) legitimately
     keeps BOTH widths — the auto-width rule cannot pick a per-operand width
     across two symbolic operands — and its required site comment is the FIXED
     string `// widths pinned: mem-to-mem two-symbolic operands`
     (`vblank.emp:97/123/125` is the precedent; this exact string is now
     canonical). A kept mem-to-mem `.w` pin WITHOUT it is an uncommented-pin miss
     (2026-07-29 audit B3: `section.emp` ×2 + `tile_cache.emp` ×6 carried the
     correct pins with no comment — fixed the same parcel as the abs-EA cleanup).
  3. Bare-symbol width-rule spellings complete — including inside
     comptime-fn / asm-template bodies.
  4. Brace-indent file-wide.
  5. The idiom list walked item-by-item (`Sst.field`,
     `OverlayV.field(aN)` for a per-object `vars` overlay's field access — the
     SST-custom-window overlay's always-on `[overlay.window-overflow]` check
     REPLACES the AS `objvarsCheck`, and a field whose bare name would shadow a
     direct struct field (`Sst`'s `angle`) is renamed on the overlay side
     (t31 — `orbit_angle`; the overlay is single-consumer with no surviving AS
     truth, so the rename carries no drift guard; joined the list at t31, the
     2nd/3rd overlay consumers after t29's DplcV / t30's TEmitterV debut),
     bareword
     `winptr`/`bankid`, label-in-immediate, `dc.l <label>` for an absolute
     symbol pointer in raw `dc` position (t25 — vector/jump table entries;
     register-class words stay loud), `raise_exception "<msg>" [, <flag>]` for a
     CPU exception/trap vector handler (t25 — the frame-less `__ErrorMessage`
     shape, sibling to `raise_error`),
     a trailing LOCAL label (`.code_end:`) for an END-OF-CODE / end-of-table
     marker inside a proc body (t27 — no global-label-in-proc-body form parses;
     the local resolves to the same address as the AS-side global and, absent an
     external consumer, needs no export; z80_init's precedent),
     OMIT the register contract entirely — never write empty `clobbers()` — on a
     proc whose CPU has NO contract model yet (t27 — `(cpu: z80)` procs are
     skipped by the 68k clobber closure until rung 2; empty `clobbers()` reads as
     the "verified: touches nothing" LICENSE, a FALSE claim for a code proc that
     clobbers registers, whereas an omitted clause is `clobbers=None` = "no
     contract declared", honest until the CPU's contract model lands; a PURE-DATA
     proc that genuinely writes no register KEEPS the honest empty `clobbers()` —
     seq_opcode_tab/dac_sample_tab vs z80_init's idle code),
     the COMMUTED `<const> + .local` immediate spelling (constant FIRST) wherever
     a leading-`.` local label would otherwise START an immediate expression
     (t27 — `ld de, 1 + .code_end`, NOT `.code_end + 1`; a leftmost `.local` in an
     immediate expr WAS parsed raw/unmangled → a dangling-symbol link bug, so
     commute — `+` is commutative, byte-identical — with a site comment; CPU-agnostic.
     **OPTIONAL as of t28 P3** — the leftmost-local-label parser fix LANDED (eval_path
     routes a leading-`.` path through eval_local_label's owner-mangling, so `.x + 1`
     and `1 + .x` now emit identical bytes; ledger row 1610 closed). The commuted
     spelling STAYS the house preference (a leftmost `.local` still reads oddly), but
     it is no longer load-bearing for correctness — drop the "until the fix lands"
     site comments at next touch),
     typed VDP fns, contract
     reglists in movem-RANGE form — `clobbers(d0-d7/a0-a4)`, not comma
     enumeration, wherever a ≥2 contiguous run exists (C1 item 2
     grammar; added 2026-07-15 after t15 shipped three enumerated
     13-register contracts — the form existed in the corpus since
     sound_api but was never ON this list; Volence's catch)
     — and the contract clause ORDER + register-group SEPARATOR are a clean
     CPU split, precedent-only until now (2026-07-29 audit A1): **68k** =
     `clobbers` → `out` → `preserves`, `falls_into` LAST, register groups joined
     by `/` (`clobbers(d0-d4/a1-a2)`), NEVER comma; **Z80** = `out` → `clobbers`
     → `preserves`, named registers joined by `,` (`out(a) clobbers(f)
     preserves(bc, de, hl)` — af/bc/de/hl are pairs, not ranges). A 68k contract
     with comma-separated groups, or either CPU carrying the other's clause
     order, is a step-2 miss (the pervasive 68k comma-group spelling — ~40 procs,
     audit B5 — is LEDGERED as its own byte-neutral sweep, un-gated now that this
     rule is TEXT; do not fold it speculatively into an unrelated parcel),
     **Absolute RAM/ROM EA = BARE symbol, auto-width — a FIRST-CLASS idiom, not a
     boundary footnote (promoted 2026-07-29 — the motivating Volence catch, see the
     META rule above).** Write `move.w Camera_X, d0` / `lea DPLC_Sonic, a2` /
     `tst.b VBlank_Ready`; the width rule picks abs.w/abs.l from the RESOLVED value
     (RAM `$FFFFxxxx` → .w, ROM / Z80-bus-window `≥$8000` → .l). The width MAY be
     spelled in a trailing `// (Sym).w` / `// (Sym).l` TWIN-NOTE comment
     (`vblank.emp:37`, the engine precedent) — that comment is NOT a code pin. An
     explicit `(Sym).w` / `(Sym).l` IN CODE is a MISS unless it is one of the
     NAMED pinned exceptions, each carrying its site comment:
     (1) mem-to-mem two-symbolic operands (item 2 above; the t15 pins);
     (2) `[lower.imm-link]` — a `#extern(...)` link-immediate source combined with
     a relaxable symbolic dest (core.emp's 4 sites, row 1046 item 2);
     (3) a movem abs seam (boot.emp:210);
     (4) the trailing-`(d16,An)` form with no bare lowering yet (parallax.emp:342);
     (5) a TRAILING-`align` structural pin — a bare abs EA lowers to a `RelaxAbsSym`
     (size-relaxable), and a later `align` / table `item_align` in the SAME section
     refuses a provisional pad position (`[align.provisional]`); so the abs EAs
     (together with the section's branches) stay explicit-width to keep every
     preceding size fixed (`game_debug.emp:105-113`, the byte-proven t26 precedent —
     its 14 `(X).w` sites are divergence-WITH-REASON, NOT drift, and were correctly
     KEPT by the style-bare-abs-ea parcel). Original link-base rule (added t16,
     2026-07-15, row 1004): absolute-EA over a LINK-TIME base spelled BARE
     `sym + const`, never the operand-override `(sym + const).w`/`.l` (the override
     comptime-folds and can't defer a link base — GENUINELY-UNBUILT; section.emp:303
     the byte-proven precedent). MOTIVATING DRIFT (the catch): game-side files went
     explicit-width for three tranches undetected — `game_debug.emp` 14 (kept-pinned,
     exception 5) / `test_churn.emp` 1 / `player_common.emp` 20 / `sonic.emp` 2 /
     `sound_debug.emp` 5 — because the rule was precedent-only, …
     Sec/Act ROM-descriptor field access — `use engine.structs` +
     `Sec.field(aN)` reg-relative displacements / `offsetof(Sec, f)` /
     `offsetof(Act, f)` (the `.field`-in-disp sugar does NOT compose inside
     displacement arithmetic — a sub-field byte reads via a named
     `offsetof(...)+N` const SHARED in engine.structs: `Act_grid_w_lo` /
     `Act_grid_h_lo` = `offsetof(Act, grid_*) + 1`, read by all 3 consumers,
     row 1068); file-local
     Sec/Act offset-const mirrors are EXTINCT — the class the 2026-07-16
     shared-struct-module batch killed (row 1051, the ratifying change).
     TILE_CACHE_{COLS,ROWS,STRIDE,NT_SIZE} come from the shared
     engine.constants twin (`use engine.constants`), never a file-local
     mirror (same batch, item 6).) with
     explicit not-applicable-because outcomes — "checked against the
     list" is the deliverable, silence is not.
  6. **The type-layer walk (Volence-ratified 2026-07-23, post-G5):**
     registers/params/outs carrying DOMAIN values — ids, indices, axes,
     coordinates, anything where two same-width values could swap or mix
     silently. Adopt existing newtypes (`types.emp` family: GridX/GridY/
     SectionId, Coord/Velocity, ObjRoutine; growing via item-13) at proc
     signatures (`(d2: GridX)`, `out(d0: SectionId)`) and `as`-bless the
     true construction sites. The value-flow test: values that are MOVED
     and COMPARED type cheaply under the strict-degrade lattice; values
     that live in shift/add chains wait for arithmetic-preservation
     (A4-i) — LEDGER those as candidates instead of forcing ceremony.
     An untyped domain value in a modernized file is a MISS unless the
     packet logs why (compute-heavy · cross-.asm seam · verifier gap,
     e.g. the FlatIDXY.d2 conditional-save pattern).
  7. **Module-header requirements (codified 2026-07-29).** A module header
     states (a) its ROLE, (b) gate + canonical-emission status, (c) EVERY
     hardware/bank/timing contract the code relies on — this is C3's audit
     surface, so the claims must be PRESENT and CHECKABLE, and (d) any layout
     the code hard-codes. A header making NO checkable claim on a
     hardware-touching file is itself a miss. Precedents: the
     sound_debug.emp / seq_opcode_tab.emp / dac_sample_tab.emp headers; C3's
     t33 catch (a DAC-loop timing claim carried verbatim from the `.asm`
     that no longer held — a header claim rots exactly like any other
     comment-claim, so step-3(b)'s comment-claim audit runs the header too).
  8. **The symbol-resolution ladder (codified 2026-07-29).** Re-home every AS
     symbol reference in PREFERENCE ORDER: module-to-module `import` → bare
     link symbol → `extern()` const WITH its drift guard + kill row → `-D`
     define carrier. `extern proc` is PROHIBITED — its opaque-leaf trust HIDES
     transitive clobbers (the whole point of porting the body is to verify the
     closure, item 9) — the ONE exception is a boundary decl to a
     not-yet-ported callee, which carries a DIE-AT-PORT kill row. Precedents:
     the t26 extern census (the corpus drops to a single honest leaf); the t28
     zero-`extern proc` headline; t34's `Player_AtLedgeEdge` (the surviving
     player_sensors callee — the corpus's SOLE `extern proc`, its die-at-port
     row satisfied at P4).
  9. **Honest-contract derivation (codified 2026-07-29).** `clobbers` =
     callee-union ∪ locally-written, VERIFIED by the closure gate (not
     hand-declared). THE DIRECTION RULE: over-claiming `clobbers` is the SAFE
     direction (callers assume worse, so a superset never breaks a caller);
     NEVER declare a `preserves` the checker cannot verify — an unverifiable
     `preserves` is a manual-honor claim, worse epistemics than a conservative
     over-claim. Precedents: t26's extern-hid-clobber catch (`Debug_MusicToggle`
     trusted the twin's hand comment → d3/d4/a0 leaked transitively via
     Sound_PlayMusic until the ported body closed the union); the t33
     Fm_ReparkDac ruling (KEEP conservative `clobbers(af)` + a site comment —
     `f` in fact survives, but the checker doesn't track flag writes, so a lone
     `preserves(f)` would be unverifiable manual-honor; the six psg/fm header
     over-claims the checker caught were ALL in the safe direction).
  10. **`as`-bless placement (codified 2026-07-29).** `as Type` goes on the
     value-PRODUCING instruction, not the consuming call site; when a wave
     changes the producer, the bless RE-HOMES with it. Precedent: t31's
     `andi.w #$FF, d0` cut re-homed the `as Angle` bless onto
     `addq.b #CHILD_ORBIT_SPEED, d0 as Angle` (the byte add wraps mod 256 →
     it is now the instruction that produces the value flowing to
     GetSineCosine's typed `Angle` param).
  11. **The noticing clause**: does THIS file suggest a house-format item
     the list lacks? Step 2 is SELF-EXTENDING — the campaign iterates
     the format file-by-file, so a spelling this file does awkwardly is
     a candidate list entry; propose it in the packet, ratified
     additions join this checklist.

  **Forced-spelling site comments (codified 2026-07-29 — generalizes the
  per-instance entries in item 5).** Any spelling FORCED by a ledgered
  frontend/parser limitation carries a site comment NAMING the ledger row, so
  the workaround is greppable and DIES when the row does. Instances: the
  COMMUTED `<const> + .local` immediate (t27, row 1610 — now optional, the fix
  landed) and the parenthesized `(ix+(field+k))` compound displacement (t32 —
  the flat-form parser fix is deferred, so this is still load-bearing). The
  rule outlives any one instance: the next parser sharp-edge worked around
  byte-identically inherits the same name-the-row discipline.

  **The feed-forward rule (Volence-ratified 2026-07-15):** every language
  feature or construct that ships with a call-site SPELLING implication
  adds its line to the step-2 checklist IN THE SAME CHANGE — the kill-row
  same-commit discipline applied to the house format. The feature's
  shipping packet names the new spelling as house format; future ports
  convert at port time; prior files join the at-next-touch backlog via
  the step-6 sweep. (`jbra`/`jbsr`, `assert`, `Sst.field`, and the typed
  VDP interface all followed this path informally — it is now the rule,
  so step 3(a)'s discoveries compound into step 2 instead of relying on
  memory.)

**Step 3 — Retrospect**: three explicit deliverables —
  (a) **language/format asks**: what did this port need that the language
      or the house format lacks or does awkwardly? (The campaign's main
      feature-discovery engine — a named output, not a side effect.)
  (b) reads-wrong / could-be-better findings;
  (c) new mirrors → kill-list rows, gaps → ledger.

  **The step-3(a) interrogation** (added 2026-07-11: pain speaks for
  itself — OPPORTUNITY doesn't. The table-for-sfx_bank class was found by
  looking at WORKING code and asking "why is this so long?", not by
  hitting a wall. Run EACH; outcomes named in the packet):
  - **Ceremony scan** — where does the file spend many lines saying
    something simple? Lines-per-intent is the signal (sfx_bank: 197
    lines for "nine SFX entries"). A high ratio is a construct/DSL
    candidate even though every line works.
  - **Comment-as-compensation** — a comment explaining WHAT the code
    does (not why) is the language failing to say it in code; a
    RECURRING what-comment shape is an ask (the FSTRING-transliteration
    class: 10 lines of comment per site because the language had no
    `assert`).
  - **Escape-hatch census** — every `extern()`, call-expr escape,
    manual `ensure()` drift-lock, and transliteration block is a place
    the language forced a detour. Count them BY SHAPE; recurring shapes
    are asks with demand data attached (they accumulate across
    tranches via the ledger).
  - **Domain-type scan** — raw ints flowing where a newtype would catch
    real mistakes (Angle, SubPixel/Speed, VramTile — the
    newtype-candidates list). FP-taste lens, gated by adoption-over-
    cleverness: a type earns its place by catching errors or naming
    intent at call sites, not by existing.
  - **Noticing clause (2026-07-15)** — did this file expose an
    opportunity CLASS no line above covers? Propose the new
    interrogation line in the packet; ratified lines join this list
    (every line above was born from a specific miss — the list is not
    finished).

  **The step-3(b) interrogation** (added 2026-07-11, same rationale as
  step 5's: reads-wrong is a checklist, not a vibe. Run EACH; outcomes
  named in the packet):
  - **Comment-claim audit** — every comment that makes a CLAIM ("no X
    can happen yet", "equivalent to Y", "safe because Z") is verified
    against the code AS IT NOW STANDS; a false or half-true claim is a
    finding (the "no band can overflow yet" class — true for the check,
    false for the bookkeeping).
  - **Contract audit** — every proc header's In/Out/Clobbers/Preserves
    matches actual register usage including transitive callees; a
    repurposed register is documented AT the repurpose site.
  - **Name audit** — labels and consts say what they do NOW (a
    `.budget_ok` that also means skipped-the-commit is a rename
    candidate).
  - **Magic-number audit** — every bare literal is a named const or
    carries a site comment saying what it encodes.
  - **Cold-reader test** — trace one frame/call through the file using
    only headers and comments; every point where you must read the
    implementation to know what happens next is a finding.
  - **Codename-reference audit (2026-07-15 — the noticing clause's
    first addition, from the corpus back-track)** — a comment may cite
    a DURABLE anchor (spec §, kill-list row, a named design doc); it
    may NOT justify by session codename ("item 11", "retro-fix batch
    2", "finding 3", "A1/A2", "tranche 11") — a cold reader can't
    resolve those. Replace the codename with the behavioral reason
    (usually already adjacent in the same comment); history lives in
    commits and notes. Ratified earlier as the exhibit-comment rule;
    the 2026-07-15 audit found ~40 sites / 16 files (ledgered,
    at-next-touch).
  - **Noticing clause (2026-07-15)** — did this file expose a
    reads-wrong CLASS no line above covers? Propose it in the packet;
    ratified lines join this list.

**Register-contract convention (Volence-ratified 2026-07-12, retro-audit):
`clobbers()` is an EXHAUSTIVE LICENSE** — everything not listed is
contractually preserved and callers MAY rely on it (this is what the
S2-D6 checked-clobbers lint will eventually verify). `preserves()` adds
movem-ENFORCED emphasis for hot reliances, but its absence does not
demote a non-clobbered register to "incidental". Prose that says a
non-clobbered register is "not a guarantee — do not rely" (the old
sound_api language) is NONCONFORMANT and gets rewritten on touch. The
step-3(b) contract audit checks BOTH directions: the body stays inside
its license, and header prose doesn't disclaim what the license grants.

**The macro-port rule (Volence-ratified 2026-07-15, from the t15 vdpComm
gate): a donor macro's .emp counterpart is an INTERFACE REDESIGN, not a
transliteration.** Applies whenever a port builds the .emp counterpart of
an AS macro or AS `function` — at step 1 (demanded) or step 4 (build).
The byte gate binds the counterpart's EMITTED BYTES against the twin's
expansion; it never binds the signature — for a comptime-fn those are
independent axes, so the ergonomics are free. Accepting the donor's
untyped parameter list by default is the trap this rule closes
("there's a macro that works this way, let's just accept it").
Precedents: `assert`/`raise_error` replaced the debugger.asm macro tower
outright; t14 `objdef()` upgraded the macro's RUNTIME priority fatal to
a COMPILE error via `u8 where 0..7`; t15 `vdpComm`/`vdp_comm_reg` is the
worked example (typed target/op sum types over raw `%100001`-class int
consts). Per macro, answer in the design note:
  - **Wrong-input scan** — what does this interface let a caller get
    WRONG that types prevent? Closed vocabularies → comptime sum types
    (exhaustive match to the encodings INSIDE the fn; the donor's int
    consts become implementation detail, drift-locked against the .asm
    truth). Ranges → refinements. Unit-bearing ints → newtypes. Bare
    boolean flags → default parameters or two named fns.
  - **Guard upgrade** — every expansion-time `if/error` and runtime
    fatal in the donor macro either becomes a comptime error/`ensure`,
    or dies because the type design made it unrepresentable; the note
    names which guards died to types.
  - **First-consumer duty** — the first port to touch a shared macro
    designs the interface every later consumer inherits; shipping an
    int passthrough "for now" and retrofitting at the 2nd consumer is
    double work in the wrong order. Under schedule pressure the
    fallback is typed-interface-with-plain-internals, never
    interface-transliteration.
  - **Taste gates** — the call site must READ better (adoption over
    cleverness), not merely differ; don't model restrictions the
    hardware/domain doesn't have; the AS twin keeps its macro spelling
    (lockstep is byte-level, not text-level — sprites/animate
    precedent).

**Step 4 — Construct pass** (Volence-ratified 2026-07-11): the same reflex
as the demanded-features law, widened from language PRIMITIVES to reusable
MACROS/CONSTRUCTS — build the toolbox up WHILE live-porting so the corpus
compounds instead of re-hand-rolling shapes. Step 3(a)'s interrogation
finds LANGUAGE-level opportunity (ceremony, escape hatches, types); this
pass finds the CONSTRUCT/macro level — code that WORKS but is repetitive
or patterned emits no pain signal, so the looking is made un-skippable
here too. **Typed signatures are part of the construct toolbox
(2026-07-23, post-G5): a new comptime fn / macro / construct whose
params or results are domain values takes and returns the NEWTYPE, not
the raw width — building an untyped construct over a typed domain
re-opens the mix-up class the type closed. If the domain has no newtype
yet, that's an item-13 candidate to ledger, not a reason to ship raw.** **Every twin mirror this pass creates (a comptime-fn whose AS
twin spells the shape inline, a mirrored const block, any scaffolding
seam) adds its twin-scaffolding kill-list row IN THE SAME COMMIT — a
mirror without a kill condition is unfinished (made explicit 2026-07-12
after the t12 clear_slot_bitmasks miss).** Scan for repeated /
patterned emission; for each, one of FOUR verbs (size-gated, byte-neutral
by default):
  (a) **adopt** — an existing construct fits (`offsets` / a comptime-fn
      helper / `dispatch` / `table`) → convert here;
  (b) **build** — no construct fits and it's SMALL (a comptime-fn helper,
      `clear_longs`/`rep` class) → build it in-port, minutes, byte-neutral.
      If the build is the counterpart of a donor macro, the macro-port
      rule (above) governs its interface.
      Readability ALONE justifies a small helper — "name the idiom" is a
      valid build reason even at 1-2 sites (Volence, 2026-07-11: using
      the language well is the point, not only dedup) — gated by taste:
      the call site must read BETTER (state intent), not merely shorter;
  (c) **ask** — no construct fits and it's BIG (new grammar/lowering,
      `table` class) → it becomes a step-3(a) ask, its own design + build;
      do NOT hand-build a stopgap;
  (d) **delete** — DEAD code. "No callers" ≠ "dead": *incidental* dead code
      (orphaned/obsoleted by OUR work) → cut (surfaced at the merge gate);
      *deliberate/feature* dead code (forward-scaffolding, an alternate
      path, an API awaiting its consumer) → FLAG to Volence first, never
      auto-cut (the `AnimateSprite_PerFrame` precedent). When ambiguous →
      treat as feature → flag. Cross-check the kill-list (kill-conditions).

  **The construct inventory** (the "adopt" checklist — check the file's
  patterns against EACH; don't work from memory. Spec §10 is the canonical
  list; this is the working cheat-sheet, keep it current as constructs ship):
  - `offsets` — `dc.w Target-Base` self-relative word tables (dense ordinal).
  - `table` (D2.36) — counted/sentinel/SPARSE keyed collection: sparse
    `{id: ptr}` blob banks (sfx_bank), count-header record lists (PLC lists,
    the six back-patch macros). NOT dense-conditional-multi-cell yet (mt_bank
    gap, ledgered).
  - `dispatch` (D2.21) — computed state/jump dispatch (encoding-agnostic).
  - `assert` / `raise_error` / `raise_exception` (2026-07-12; raise_exception t25)
    — DEBUG/error diagnostics: `assert.<w> src, cond [, dest]` (self-gates to zero
    bytes in the plain shape), unconditional `raise_error "<fstring>"` (a DELIBERATE
    raise from normal code — simulates the exception frame with `pea self(pc)` +
    `move.w sr`), and `raise_exception "<fstring>" [, <flag>]...` (t25 — the
    EXCEPTION-VECTOR counterpart, i.e. vladikcomper's `__ErrorMessage`: NO frame
    push because the CPU pushed SR+PC when it vectored here; used by CPU
    exception/trap vector handlers; the flag list is the error-handler per-message
    set, `address_error` today). Replaces hand-spelled debugger.asm macro-tower
    transliterations (kill row 16). `ifdebug <x>` ports to `if DEBUG == 1 { <x> }`.
    The `src` must be a register (load first, inside the `if DEBUG == 1` so the load
    itself is gated). Keep operand spellings identical to the AS twin (the auto-
    message embeds them). rings/core are the shipped assert/raise consumers;
    error_handler is the raise_exception consumer.
  - comptime-fn helpers — repeated-emission templates: `clear_longs` (unrolled
    fill), `rep` (repeated bytes), `reload_anim_timer`/`perform_dplc`
    (instruction templates), `aabb_axis_test`, `ojz_sec` (validating record
    constructor), `objroutine(label)` (label − ObjCodeBase); loop templates via
    `{code}` splice (skeleton-with-holes — the loop skeleton's label + branch
    live in ONE `asm{}` block, flip/variant-dependent segments are label-free
    `{term()}` splice holes; `emit_piece_loop` is the reference).
  - contracts — `clobbers`/`preserves`/`out` (reglist form), `let rN: Type`
    (body-position typed register).
  - spelling idioms (step 2, not this pass) — bare Bcc, `jbra`/`jbsr`,
    `Sst.field`, bareword `bankid`/`winptr`, label-in-immediate, `dc.l <label>`
    (t25 — an absolute symbol pointer in raw `dc` position, e.g. a vector/jump
    table entry; register-class words stay a loud error).

  Also scan for STRUCTURAL clones — N-variant duplicated bodies
  differing in one or two terms (the `emit_piece_loop` class): they are
  adopt/build/ask candidates even when each copy is individually clean
  code, and the varying terms name the template's parameters.

**Step 5 — Optimize**: the real question — is this ENGINE CODE actually
good? Algorithmic/cycle-level, not assembler spelling. Behavior-affecting
changes live here and need LIVE verification (oracle) on top of the
lockstep + re-pin mechanics. "No changes, recorded why" is a valid
outcome — but only AFTER the interrogation below, with each line's
outcome named in the packet. **Type-layer rider (2026-07-23, post-G5):
an optimization that reshapes register flows RE-CHECKS the blessings it
moves (an `as`-bless belongs on the new producing instruction, and a
reshuffle that routes a domain value through scratch arithmetic degrades
its type — re-bless or re-route); and a mix-up class the optimization
newly exposes (two same-width domain values now adjacent in registers)
is a newtype candidate to adopt or ledger, same bar as step 2 item 6.**

  **Oracle A/B for byte-gate-blind changes (codified 2026-07-29).** Any edit
  whose effect a byte gate CANNOT see — a behavior fix, a runtime-state claim,
  an inherited-flag change, a lifecycle change — requires an emulator A/B with
  NAMED, frame-anchored observations in the packet, not a bare "verified on
  oracle". The byte gate is doubly blind on a faithful-but-wrong port: both
  twins agree, so identity proves nothing about correctness. Precedents:
  t24's three oracle-verified children fixes (the effect `parent_ptr` dead
  store gone at the same slot; the COORDMODE inherit measured `render_flags`
  `$00→$08` the instant the creator returns; the chain-contract asserts); the
  fix-test-parent-lifecycle A/B (the `DeleteChildren` cascade FIRES — 3 parents
  + children expire ~3 s, dynamic slots reclaimed, scene healthy — a
  behavior fix the gate could not see because the twin carried the same dead
  self-destruct path).

**PARCEL-SCOPE AMENDMENT (2026-07-24, Volence-directed; corrected same
day):** in-tranche step 5 runs FULL — the interrogation below plus
threshold-ruled cuts (the t18 H2 pattern: profile first, cut ≥~1k cyc/frame
steady-state with live A/B, log-and-skip below with numbers). A tranche's
own file is already paying the ripple/re-pin ceremony, so its cuts ride
nearly free. What is DEFERRED until the conversion campaign completes is
**standalone optimization/hardening parcels on already-ported files** (the
sprites-hardening kind): each such parcel pays its own full ceremony +
merge-queue coordination, and the same work costs roughly half after asl
retires (one source file, no twin lockstep, no re-pin). Those park with a
STATUS banner on their brief and their ledger rows stay open; the
post-conversion optimization sweep runs them off the logged backlog with
full profiler context.

  **The step-5 interrogation** (added 2026-07-11 after the t11 sprites
  review: a second look found real items behind a "no changes" verdict.
  "Looks hand-optimized" is ANCHORING, not analysis; "no profiler" blocks
  measurement, never inspection. Run EACH line, per hot proc):
  - **Invariant ladder** — classify every loop instruction by the WIDEST
    scope it's invariant over (iteration → object → band → frame →
    build); anything sitting below its scope is a hoist/fold candidate
    (the camera-bias class: per-piece `addi #128` folds into the
    per-frame camera read).
  - **Counter/cache audit** — for every counter, cache, or budget: list
    ALL writers and ALL readers; every path that consumes the guarded
    resource must charge it, or the asymmetry is documented as intended
    (the scanline-budget class: an early-out skipped the COMMIT; a
    sibling path skipped the charge entirely).
  - **Guard-coverage audit** — for every limit/safety check: enumerate
    every emission path; is the check on all of them? Name which checks
    are LOAD-BEARING vs redundant (the dbeq cap-net class: sole guard
    for uncached counts — protect it from future "cleanup").
  - **Hardware cross-check** — every VDP/hardware-facing behavior gets
    checked against the documented quirks (sprite-mask first-sprite-
    on-line exemption, per-line sprite/pixel limits, DMA boundaries…);
    what can't be verified statically becomes a named oracle probe in
    the packet, not a silent assumption.
  - **Silent-tradeoff comments** — every accepted behavioral compromise
    (cascade-down under overflow, a skipped call that's coincidentally
    equivalent, unconditional fairness cycling) gets a site comment
    saying it is CHOSEN — an uncommented compromise is a finding.
  - **Debug-growth boundary check (2026-07-24, t24)** — any change that
    grows the DEBUG shape (asserts, `if DEBUG == 1` blocks, message blobs)
    checks whether it pushed an engine symbol consumed by OBJECT-BANK code
    across `$8000`: the AS width rule then widens the bank's
    `jsr (Sym).w` call sites to `abs.l` (+2 each) and slides the whole debug
    object bank, which breaks every fixture that assumes the two banks
    coincide. Cheapest check: an object-bank symbol's plain and debug pins
    coincide iff the bank did not slide. Full bar + the four fixture families
    in the gap-ledger ("the $8000 abs.w/abs.l bar"). Assert blobs are budgeted
    accordingly — an assert on a zero-caller proc can never fire and still
    spends the bytes.
  - **Noticing clause (2026-07-15)** — did this file expose an
    optimization or hazard CLASS no line above covers? Propose it in
    the packet; ratified lines join this list.

  **Hot-path second look**: files on the per-frame hot path (render,
  physics, object/frame loop) get a Fable review pass before the merge
  gate — the checklist raises the floor; the second look is the ceiling.

**Loop until dry**: after step 5, retrospect again; anything found →
construct-pass/optimize again. The exit condition is a FULL 3→4→5 pass
with zero findings at ALL THREE steps (empty retrospect AND empty
construct pass AND no-changes step 5) — see THE SHAPE box at the top;
an empty step 3 alone does not exit the loop.

  **The dry-panel rule (Volence-ratified 2026-07-23; first tranche =
  t18):** DRY is no longer self-declared. When the porting agent's own
  full pass comes up empty, it dispatches a PANEL of 2-4 FRESH read-only
  subagents, each running ONE lens over the ported file + packet.
  **Composition is WEIGHTED toward step 5 (Volence-ratified same day):
  A×1 · B×1-2 · C×2-3** — the campaign's gate-blind bugs all lived in
  step-5 territory, so that lens family gets the most independent eyes:
  **Lens A ×1** (step-3 flavored) — cold reader for ceremony pain and
  language asks the porter has gone blind to. **Lens B ×1-2** (step-4
  flavored) — corpus-pattern matcher; at ×2 split the two directions
  (B1: does this file re-hand-roll an existing construct; B2: does the
  corpus re-hand-roll this file's new shapes — the t17 step-6 vdp-dup
  class). **Lens C ×2-3** (step-5 flavored), ALWAYS ≥2, with distinct
  sub-lenses: **C1** cycle/perf auditor (hot-path costs, budget fit,
  cheaper addressing/table forms); **C2** correctness-hazard hunter
  (the gate-blind checklist: hand-computed strides · CC-clobber between
  test and Bcc · loop-backs provably fire · conditional save/restore
  reliance); **C3** (when the file touches VDP/DMA/interrupts/bus)
  hardware-timing lens — VBlank budget, bus contention, IRQ ordering,
  the "math checks out but timing doesn't" class. Scale within the
  ranges by file size and heat; the floor is A1+B1+C1+C2.
  Panel findings land in the packet and are ADJUDICATED AT THE GATE;
  any real finding re-opens the cycle. **DRY = a panel round returning
  nothing new.** One panel round per dry claim (cost-bounded, not
  continuous); panel agents are read-only analysts — they report, never
  edit. Rationale: the second-reviewer sweep went 7/7 on bugs the porter
  walked past, and t17's own step-6 caught the agent's own duplicate —
  an agent cannot see its own blind spots by looking harder; a fresh
  lens is structural, not optional.

  **Panel-activation rules (codified 2026-07-29):** **C3 ACTIVATES whenever
  ported PROSE makes hardware claims** — not only when the code touches the
  bus. A header carried from the `.asm` asserting a timing/bank/contention
  fact is a hardware claim, so C3 audits it (the t27 ruling: overseer-activated
  C3 verified 10/10 header claims against the resident tree; the t33 payoff:
  C3 caught the one stale DAC-loop timing claim the port inherited). **A C1
  decision is a FLAGGED CALL, never silent** — when the porter rules C1
  inactive (e.g. a change is on a debug/human-timescale path, not the hot
  path), it NAMES the sites and logs the ruling in the packet for gate review,
  it does not quietly skip the cycle lens (t29 onward — a named-basis C1-inactive
  ruling, reversible at the gate, is the standing form).

**Step 6 — Corpus sweep** (Volence-ratified 2026-07-11; was the old
in-loop step-4 back-propagate, pulled OUT to a single final GATED pass —
"one combined wave, not two", stated plainly): ANY new addition this
tranche made that PRIOR FILES could use — a format idiom, an adopted/built
macro/construct, OR an optimization — triggers a sweep of ALL
previously-ported `.emp` files. **Retrofit where clean; LEDGER where
blocked** (a site waiting on an unshipped dependency gets a ledger row, not
a forced conversion — else the sweep stalls). Trigger is "new thing PRIOR
FILES could use" (something unique to this one file earns no sweep).
Verification differs by kind: construct-adoption is usually byte-neutral
(cheap, byte gate); an OPTIMIZATION sweep changes bytes (re-pin +
live-verify per site). This trigger also closes the hole a per-port step
can't reach: constructs ship AFTER files are ported (a standalone build
like `table` has no in-tranche step-4), so the obligation attaches to the
ADDITION, whenever/however it ships.

  **Pattern-enumeration amendment (Volence-ratified 2026-07-12, from the
  retro-audit's three confirmed misses — A1→rings, preserves()→animate,
  compact-comment→mid-dispatch):** the sweep is an ENUMERATION, not a
  judgment pass. When the addition is an optimization, contract
  capability, or invalidated assumption, grep the SHAPE it addresses
  across the whole corpus (the pattern's operands/idiom, e.g.
  `VDP_SPRITE_._OFFSET` for the A1 fold) and name EVERY site's outcome
  (retrofitted / ledgered / not-an-instance) in the packet. "I updated
  the file at hand" is a file-scoped fix, not a sweep.

**Standing patterns (codified 2026-07-29 — the game-side arc's reusable
mechanics, so a new tranche inherits the shape instead of re-deriving it):**

  **The standing-patterns registry.**
  - **Multi-file tranche scaffolding (t29):** PER-FILE gates
    (`SIGIL_EMP_<OBJ>` each — cleaner per-lane region pins), ONE shared test
    file per tranche (`test_g1_objects_port.rs`), ONE mixed-build fn per
    tranche (`build_mixed_tranche29_rom` + its assemble-AS-side sibling). The
    combined-gate template (one gate / two files) is SUPERSEDED for multi-file
    tranches.
  - **The INTERNAL-gate keystone (t34):** a KEYSTONE file whose zero-byte
    header (equates / a `vars` overlay / macros) feeds SURVIVING AS consumers
    gets an INTERNAL gate — a gate boundary inside the file with a gate-off
    code twin + an else-`org` resume arm — not a whole-file gate, so the
    header's definitions stay visible to those AS consumers in BOTH build
    shapes (player_common: the PlayerV overlay + `_pl_*`/`PPHYS_*` equates +
    4 macros feeding player_ground/air/sensors; else-org `$10452→$10448`).
  - **The guarded-vs-unguarded overlay decision:** guard EXACTLY the overlay
    fields that a surviving AS file still reads (t29 DplcV — the row-61 class:
    drift guards resolve against the real AS tree); ZERO guards for a
    single-consumer overlay with no surviving AS reader (t31 TParentV/TOrbitChildV
    — the row-66 class; a grep of the whole AS `games/` tree finding no reader
    is the license, and a shadowing rename like `angle`→`orbit_angle` carries no
    guard either).

  **The wave-ripple checklist (t34 census — the canonical instance every
  byte-moving game wave inherits; suite-green is the completeness proof):**
  1. reference REBUILD both shapes (plain + DEBUG=1); repin.
  2. EVERY org-resume arm — the main.asm gate-resume orgs AND the
     act_descriptor SELF-GATE class: a FIXED `org $` that RE-DOES the slide for
     everything after it until updated (it lives in a DATA file, not main.asm,
     and was the arm that broke tranche4-9 mixed builds). Sweep every `org $`
     in ALL game `.asm`, not just main.asm.
  3. the row-1257 downstream-slide sweep (the whole object-bank + data run
     shifts by the shrink amount; repin regenerates the pins, the arms below
     are HAND-checked).
  4. the 5-site hand-check: engine.inc (`org $10000` unaffected unless an
     engine region slid) · mixed_dac_rom.rs (hardcoded reference-ROM read
     windows + any self-relative objroutine LITERAL that slid) · repin_pins.rs
     · repin.toml (only if a region was added) · test-port doc-comment addresses.
  5. the $8000 bar (a DEBUG-shape growth that pushes an object-bank-consumed
     engine symbol past `$8000` widens the bank's `jsr (Sym).w` to abs.l and
     slides the whole debug bank — check plain==debug object-bank bases still
     coincide).
  6. ALL neighbour/prior-tranche gates run as CANARIES (a coincidental
     pre-slide byte match is a latent spurious pass the slide exposes — assert
     the real stored word, e.g. `TestAnimated_Main` not `TestAnimated`).
  7. a per-region delta table in the packet.
  8. **Rebuild the worktree ROMs after ANY rebase** (the t32 stale-artifact
     lesson): a rebased branch's on-disk ROMs are stale until rebuilt — verify
     CRCs against a fresh dual rebuild before trusting any gate.

**Merge**: only after a dry retrospect + the corpus sweep — checkpoint
packet to Volence, his gate, then --no-ff merge both sides + push. Every
merge to master is FINISHED code, not faithful-but-stale-idiom code.

  **Paired-state gate (Volence-ratified 2026-07-13, the mulu/13-gate
  merge night — full story in 2026-07-13-paired-state-gate-merge-packet):**
  an aeon branch is NOT mergeable until sigil's FULL strict suite runs
  green with `AEON_DIR` pointed at THAT BRANCH'S TREE — never at aeon
  master. Gating against master certifies your code against a world it
  isn't in (two same-night instances: a churn-scene `mulu` sigil's
  frontends lacked, and a +0x78 bank growth colliding with pinned .emp
  placements — both invisible until merge). Corollaries from the same
  packet: attribute EVERY delta to its cause before merging; split a
  multi-cause regression by OWNER; push coupled masters TOGETHER (no
  stale window); a fragility class gets EXTINGUISHED on its second bite,
  not re-patched; and predicted-delta lines model the change's CODE, not
  just its data.

**Packet format (Volence-ratified 2026-07-10)**: the packet ends with a
"What each pass added" section separating STEP-3 findings (asks / reads-wrong
/ kill rows / ledger) from STEP-5 findings (optimizations taken + not-taken),
PER LOOP PASS — so each look of the tranche shows what it added. Findings
that fit neither (step-1 demanded features, live bugs, probe outcomes) keep
their own headline bucket.

**The packet's step-5 (and step-3(a)/3(b)) sections ARE the filled
checklists (enforced at the gate from 2026-07-12):** one line per
interrogation item per hot proc, each with its outcome (taken /
not-taken-with-reason / not-applicable-because). A prose summary
("hot path already minimal") in place of the table does not pass the
merge gate — the t11 sprites review and the t12 blind run both showed
summaries hide items the per-line walk finds.

Keep tranches small (2-3 files): step 2 makes re-pins routine, and
short-lived branches keep the re-pin tax per-tranche instead of
compounding against master drift.
