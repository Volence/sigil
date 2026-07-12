# .emp diagnostics construct — `assert` + `raise_error` (design)

2026-07-11 · Fable · status: DRAFT awaiting Volence gate
Kills: kill-list row 16. Closes: gap-ledger "assert/diagnostics — demand 1/2".
Unblocks: entity_window.asm port (11 sites), path_swap.asm (1 site).

## 1. Problem — the transliteration tax

debugger.asm (vladikcomper MD Debugger v2.6 macro layer, 806 lines) gives AS
code one-line diagnostics: `assert.b d4, eq, #0` expands to a CCR-safe
compare + branch + an inline `RaiseError` blob (pea self / SR push / arg
pushes / jsr MDDBG__ErrorHandler / FSTRING-encoded message bytes / exit-flag
byte / jmp PagesController). `.emp` has no macro tower, so ported DEBUG
blocks are hand-spelled TRANSLITERATIONS — rings.emp carries a 24-line block
(incl. two parity-derived flag/pad constants) for what AS says in one line;
core.emp carries three more.

**Demand census (2026-07-11, full corpus, .worktrees excluded):**

| Macro | Sites | Where |
|---|---|---|
| `assert` (incl. inside `ifdebug`) | 24 | entity_window 11, s4lz 3, tile_cache 2, core.asm 3, compression_selftest 4, rings 1 |
| `ifdebug` bare-instruction | 5 | s4lz 3, tile_cache 2 (+ core.asm's 2 `bsr` wraps) |
| `RaiseError` direct | 1 | games/sonic4 path_swap.asm |
| `Console.*` / `KDebug.*` | **0** | — |

The gap-ledger entry (tranche 8) held this at "demand 1/2 — second demand
ratifies designing it; the debugger port era does." entity_window.asm is
next in the port queue with 11 sites: that is the ratifying demand. Porting
it without the construct = hand-spelling the FSTRING tower eleven times —
the most error-prone transcription work in the corpus.

## 2. Approaches considered

**(A) Port debugger.asm as a tranche.** Rejected: it's a macro library that
emits zero bytes on its own — step 1 (transcribe + byte gate) has nothing to
attach to. The runtime (error_handler.asm, a vendored 45-export blob) stays
.asm behind the link seam regardless; only the macro layer needs an .emp
answer, and that answer is a construct, not a port.

**(B) Comptime-fn prelude library (no grammar).** Rejected on four
mechanics: comptime fns take VALUES, not operands/EAs (`assert` needs `d4`,
`#MAX_LIST_ENTRIES` in operand position, spelled into the message);
expansions need hygienic local labels (`.skip`) which `Code` values can't
carry; the emission is interleaved code+data+label; and the message text is
derived from source SPELLING, which only the parser sees.

**(C) Grammar-level construct with twin-parity lowering** — like `table`
(D2.36): parser → AST → eval/validation → lowering, plus acceptance
vectors. **Chosen.** The FSTRING "compiler" lives in the lowering layer
(Rust), not in .emp comptime code — it is a deterministic encoder, and
keeping it in the construct means every call site stays one line.

## 3. Surface

Statement position, proc body. AS-parity spelling — converting a site is a
copy-paste (adoption over cleverness; expression sugar like
`assert.w d1 <u #MAX` is a post-Spec-5 bikeshed, noted §9):

```
assert.b d4, eq, #0                  // cmp form:  assert src <cond> dest
assert.w d1, lo, #MAX_LIST_ENTRIES
assert.l a0, ne, #0
assert.w d1, eq                      // tst form:  flag test on src alone
raise_error "Bad path swap!%<endl>Got: %<.b d0>"
```

- **`assert.<w> src, cond [, dest]`** — `<w>` ∈ b/w/l (required);
  `cond` ∈ the 16 Bcc condition codes (eq ne cs cc pl mi hi hs ls lo gt ge
  le lt vs vc), lowercase; `src`/`dest` are real operands, not strings.
- **`raise_error "<fstring>"`** — user-written format string; same token
  grammar as debugger.asm: literals, `%<endl>`/`%<cr>`/`%<pal0..3>`/
  `%<setw N>`-class controls, `%<.b|.w|.l operand [param]>` arguments with
  param ∈ hex (default) / dec / bin / sym / symdisp / str / signed / split /
  forced / weak.
- **`ifdebug` gets NO construct** — `.emp` already has statement-position
  `if DEBUG == 1 { ... }` (D-P2.6); the port-loop conversion rule is
  `ifdebug <x>` → `if DEBUG == 1 { <x> }`.
- **`Console.*` / `KDebug.*` / the `consoleprogram` parameter: out of
  scope** — zero corpus demand. Grammar reserves nothing; ledger rows added
  (§8). `raise_error` with a `consoleprogram` argument is a compile error
  with a steering diagnostic.

## 4. Semantics

### 4.1 Gating
`assert` self-gates: when `DEBUG != 1` it emits ZERO bytes (matches the AS
macro's `ifdef __DEBUG__`; call sites are written bare, no wrapper).
`raise_error` is UNCONDITIONAL (matches AS: RaiseError has no ifdef of its
own — path_swap's is a release-path fatal).

### 4.2 `assert` expansion (DEBUG shape) — twin-parity, in order
1. `move.w sr, -(sp)` — CCR save
2. `cmp.<w> dest, src` (cmp form) or `tst.<w> src` (tst form)
3. `b<cond>.w .skip` — **pinned .w** (generator-owned structural width: the
   branch jumps over variable-length inline data; twin-parity besides)
4. `pea *(pc)` — self-address for the handler
5. `move.w sr, -(sp)` — SR for the handler display
6. arg push for `src`: `.b` → `subq.w #2, sp` + `move.b src, 1(sp)`;
   `.w` → `move.w src, -(sp)`; `.l` → `move.l src, -(sp)`
7. `jsr (MDDBG__ErrorHandler).l`
8. inline data: the encoded auto-message (§4.4), arg descriptor byte
   (`$80|0/1/3` for b/w/l, hex), `$00` terminator
9. exit-flag byte: `_eh_return` (`$20`), OR'd with `_eh_align_offset`
   (`$80`) + one `$00` pad iff the flag lands at an EVEN offset (§4.5;
   direction corrected 2026-07-12 — build finding 1)
10. `jmp (MDDBG__ErrorHandler_PagesController).l` (extensions-enabled
    config — §7 caveat)
11. `.skip:` (hygienic, generated) then `move.w (sp)+, sr` — CCR restore

### 4.3 `raise_error` expansion
Steps 4-10 of the above, with the user's format string instead of the
auto-message, arg pushes generated per `%<...>` token in REVERSE token
order (matching `__FSTRING_GenerateArgumentsCode`), one descriptor byte per
token at its position in the string.

### 4.4 Auto-message derivation (`assert`)
Byte-for-byte the AS macro's template, built from source SPELLINGS:
`"Assertion failed:" $E0 $EC "> assert.<w> " $E8 "<src-spelling>," $EC
"<cond>" [$E8 ",<dest-spelling>"] $E0 $EA "Got: "` — cond lowercased,
operand spellings verbatim from source. **Retrofit rule: keep the .emp
operand spelling identical to the AS twin's** (`#Object_RAM` stays
`#Object_RAM`) or the message bytes diverge.

**§4.4 amendment (2026-07-12, Task-2 gate, Fable-ratified):** spellings
are VERBATIM SOURCE SUBSTRINGS — the parser retains the file text
(`Rc<str>`) and slices each operand's span at parse time. Byte-exact by
construction: any spelling the author writes (`#0`, `#$100`, `#%1010`,
`#Object_RAM`, internal whitespace) reproduces exactly, so there is NO
radix or form restriction on message-visible operands. (The plan's
slice-at-parse intent stands; the interim "slice at eval" routing rested
on a false premise — eval/lowering verifiably carry no source — and is
void. The loud-guard fallback is dead: with no token-reconstruction
step, there is nothing to guard. Scaffolding-era mechanism, ledgered.)

### 4.5 Parity rule
**(Direction corrected 2026-07-12, build finding 1 — the original said
"odd"; the byte examples were always right, the prose was inverted.)**
Offset parity is tracked across the emitted data run; if the exit-flag
byte would land at an EVEN offset, emit `flag|$80` + `$00` pad — the jmp
that follows must start word-aligned, so flag-at-even (jmp would start
odd) pads, flag-at-odd (jmp starts even) doesn't (rings: `$A0,$00` =
padded; core: `$20`, no pad — both reproduced; verified from
debugger.asm:264 + all four transliteration blocks). Final alignment
matches the macro's trailing `!align 2`.

## 5. Validation (eval/layout stage — fail loud, steer)
- unknown cond → error listing the 16 codes; missing width → error.
- `src` must be a register (dn/an) in v1 — memory forms error with
  "move to a register first" (matches corpus practice; debugger.asm's own
  parenthesised-operand limitation, rings.emp comment / AS error #1300).
  `dest` may be register or immediate (incl. `#symbol`).
- `raise_error` tokens: unknown control/param name → error with the token
  table; param byte must be ≥ `$80` (the macro's own check); `.b/.w/.l`
  arg operands limited to registers + immediates in v1, steering error
  otherwise.
- `DEBUG` undefined → error (house convention: shapes are explicit).
- `assert` outside a proc body → error.

## 6. Lowering + implementation map (table as template)
- `parser.rs` — statement-position `assert` / `raise_error` (keyword
  detection alongside the existing statement forms).
- `ast.rs` — `AssertStmt { width, src, cond, dest }`,
  `RaiseErrorStmt { fstring, args }` (args pre-parsed from the string).
- `eval/` — §5 checks; fstring token parse; message/descriptor byte
  assembly (pure function → unit-testable).
- `lower/mod.rs` — expansion emission incl. hygienic `.skip` labels, the
  parity computation, and the DEBUG-shape gate (plain shape emits nothing
  for `assert`).
- Tests: unit vectors (all 16 ccs encode; three widths; tst + cmp forms;
  parity both ways; every control token; descriptor bytes) + acceptance
  vectors (§7) + negative probes (§5 errors).

## 7. Acceptance vectors + gates
1. **rings.emp retrofit**: the 24-line transliteration block → 
   `assert.b d4, eq, #0` — DEBUG-shape byte gate stays green (the existing
   gate IS the vector).
2. **core.emp retrofit**: Debug_AssertObjLoop's three asserts (`.l`/`.w`,
   cmp forms, `#Object_RAM` symbol-immediate, the `$20`-no-pad parity case)
   → three one-liners — byte gate green.
3. Mixed-build + gate-off neutrality + negative probes per house rules.
4. **Drift-guard property preserved**: the retrofitted sites still ride the
   debug-shape byte gates, so a debugger.asm macro change still fails
   loudly — the tripwire moves from hand-copied bytes into the construct's
   vectors.
5. **Config caveat**: the emission matches this engine's debugger config
   (extensions on → `jmp PagesController` tail; `_eh_default = 0` unless
   `DEBUGGER__SHOW_SR_USP`). The lowering hard-codes this engine's config;
   a config change is a construct change (acceptable — one engine).

## 8. Bookkeeping shipped with the construct
- **Kill-list**: row 16 killed (rings/core retrofits). NEW row: the
  construct's twin-parity emission mirrors debugger.asm's token encoding +
  engine config; kill condition = Spec 5 (twins die → message format and
  the `b<cond>.w` pin are freed).
- **Gap-ledger**: close "assert/diagnostics demand 1/2" (ratified: 30
  diagnostic sites corpus-wide, entity_window era). Add: `Console.*`/`KDebug.*` construct
  (demand 0), `consoleprogram` param (demand 0), memory-operand arg push
  (demand 0), comparison-operator assert sugar (post-Spec-5 taste item).
- **Step-6 sweep obligation** attaches on ship: rings + core retrofits
  (byte-neutral, cheap gates). No other ported file has sites.

## 9. Sequencing + bikeshed
- Tranche 11 (sprites.asm, zero debugger sites) runs in parallel — no
  dependency either way.
- Build order: construct (this spec) → step-6 sweep (rings, core) →
  entity_window.asm port (tranche 12 candidate) with one-line asserts.
- path_swap.asm's `raise_error` converts whenever games-side porting
  reaches it; v1 grammar already covers it.
- Names: `assert` (matches AS, unambiguous) / `raise_error` (house
  snake_case; AS's `RaiseError` spelling reserved for the twin). Sugar
  (`assert.w d1 <u #MAX`, typed-register signedness inference via `let`)
  deliberately deferred: cc-mnemonic form is the porting workhorse; sugar
  can layer later without breaking it.
