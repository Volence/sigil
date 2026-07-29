# t25 BRIEF — the debug trio (error_handler + sound_debug port; debugger.asm RECLASSIFIED)

**Overseer: Fable (direct dispatch). Date: 2026-07-28.** Single brief carrying the
step-0 design ruling — overseer recon is DONE (all facts below verified against the
live tree and fresh listings s4.lst/s4.debug.lst, dual build of masters
aeon `4df4ad8` / sigil `26597ff`).

## THE TARGET RULING — the trio is NOT three region ports

1. **`engine/debug/debugger.asm` (807) — RECLASSIFIED, not ported.** It emits ZERO
   ROM bytes in both shapes (included at engine.inc:76 BEFORE `org 0`; contents =
   DEBUGGER__* config equates + the macro tower `ifdebug/assert/_assert/RaiseError/
   Console/_Console/KDebug/__ErrorMessage/__FSTRING_*` + `padding off`/`supmode on`;
   no top-level dc.*/instruction — structurally confirmed, and Vectors/EntryPoint sit
   at 0 in both listings). Its consumers are the `.asm` twins and game-side `.asm`
   exclusively; no `.emp` file references any of it (the `.emp` side has its own
   diag construct — kill row 16 KILLED 2026-07-12, row 21 tracks the twin-parity
   emission). `Console.*`/`KDebug.*`: ZERO invocation sites corpus-wide (gap-ledger
   line 995, demand 0). The macro tower MUST stay AS-side while any `.asm` twin
   exists, so its retirement condition IS twin retirement → **new
   POST-TWIN-RETIREMENT bucket row** (porter adds it, same commit as the ledger
   sweep): at asl retirement, DEBUGGER__* config needs an `.emp`-era home, the
   macro tower dies with the twins, and kill row 21's message-format/`b<cond>.w`
   pins are freed. The engine 68k conversion backlog drops debugger.asm by
   reclassification, not by port.

2. **`engine/debug/error_handler.asm` (258) → `error_handler.emp` — THE PORT (primary
   lane).** Un-shape-gated (its only `ifdef __DEBUG__` is the EMPTY placeholder block
   at lines 16-17), emitted in BOTH shapes at the ROM tail.

3. **`engine/debug/sound_debug.asm` (98) → `sound_debug.emp` — small lane.** Zero
   bytes in BOTH canonical shapes (triple gate `__DEBUG__`+`SOUND_DRIVER_ENABLED`+
   `SOUND_DBG_MIRROR`; third gate defaults 0 in build.sh:67-68). The port is proved
   at the MIRROR SHAPE (`DEBUG=1 SOUND_DBG_MIRROR=1`) plus canonical-emptiness both
   shapes — machinery bar below. If the mirror-shape machinery balloons past LEAN,
   STOP at the checkpoint and the overseer rules defer-vs-continue.

**After t25 the engine 68k backlog is EMPTY** (+Z80 ~5 Volence-deferred, game-side ~10).

## REGION MACHINERY (listing-derived, both shapes)

New region `error_handler` = `BusError` .. `EndOfRom`:

| Anchor | plain | debug |
|---|---|---|
| `NullInterrupt` (prev end anchor; `rte` inline at engine.inc:663-664) | 0x5CAAE | 0x5E5A8 |
| `BusError` (region start) | 0x5CAB0 | 0x5E5AA |
| `ErrorHandler` (blob start) | 0x5CC0A | 0x5E704 |
| `EndOfRom` (region end = end of `dc.w $0000`) | 0x5DB60 | 0x5F65A |

Length **0x10B0 BOTH shapes** (stub table 0x15A + blob 0xF56) — same-size, different
base; NOT a shape-split region. The MDDBG__ equ table (error_handler.asm:206-250)
emits nothing, so the region tail lands exactly on `EndOfRom` — **the byte before the
convsym `deb2` appendix**. Byte gates therefore REUSE the harness pattern
`assert_rom_matches_convsym` (crates/sigil-harness/src/lib.rs:585-660; consumers
m1d_debug_rom.rs:88, diff_s4_debug.rs:54) with the derived checksum/ROM-end-field
allowlist, and pin ASSEMBLED_LEN plain 0x5DB60 / debug 0x5F65A (UNCHANGED is a bar —
t25 is expected byte-neutral end to end, see below). Gate `SIGIL_EMP_ERROR_HANDLER`,
test `error_handler_port`. repin.toml gains the region block (5-site ripple doctrine
in force if anything moves; nothing is expected to). The $8000 bank-shift bar is
N/A here (ROM-tail region, far outside the abs.w window) — state it, don't run it.

`sound_debug` region exists ONLY at the mirror shape; canonical proof = emptiness
in both shapes (t22's `compression_selftest_plain_region_is_empty` is the template,
here needed for BOTH canonical shapes) + a mirror-shape byte gate. Precedent
machinery: `vblank_port::vblank_mirror_shape_twin_parity` (kill row 42's gate)
already builds a mirror-shape arm — extend, don't invent.

## SEAMS AND FLIPS

- **vectors.asm (stays `.asm`, out of scope)** references all 12 stub labels as
  symbolic `dc.l` (BusError/AddressError/…/ErrorExcept ×5/ErrorTrap ×32) —
  shape-INDEPENDENT spellings, listing-confirmed both shapes. Under the gate the
  labels flip to `.emp` ownership → the proven ownership-flip class (engine.inc
  contract symbols); the flip artifact is a link test proving the vector table still
  resolves in both shapes and both gate states.
- **MDDBG__ consumer census: ZERO literal references outside the two debug files.**
  Every corpus consumer (assert ×42-ish across 10 files, RaiseError ×5 across 3,
  __ErrorMessage ×12 in error_handler itself) reaches the blob through MACRO
  EXPANSION (`jsr (MDDBG__ErrorHandler).l` etc. — abs.l, so region relocation is
  displacement-safe). The MDDBG__ equ table STAYS AS-SIDE (debugger.asm's macros
  need it) and derives off the flipped `ErrorHandler` contract symbol —
  `equ ErrorHandler+$xxx` over a link-resolved base. Porter proves this resolves
  gate-on/gate-off.
- **`Sound_DebugMirror`**: sole caller vblank.emp:47-49 (comptime-gated call over an
  ungated extern decl — kill row 42). Porting the callee KILLS row 42 (decl deleted
  same-commit, gate artifact re-anchored). vblank.asm twin keeps its ifdef arm.
- **The `.emp` diag construct is the port vehicle for the 12 stubs** — NOT a
  transliteration of macro output. 10 of 12 stubs are `opts = _eh_default = 0`
  (build config has SHOW_SR_USP=0), whose expansion (args-code empty for the fixed
  strings + `jsr (MDDBG__ErrorHandler).l` + FSTRING-encoded string + flag byte
  `$20|align` + `jmp (MDDBG__ErrorHandler_PagesController).l`) is EXACTLY what
  `raise_error` lowers to today (diag.rs raise_tail:507-544 hardcodes `_eh_return`).
  Prove with a probe before leaning on it.

## THE DEMANDED FEATURE (TDD, the tranche's one construct item)

**`raise_error` options form** — demand: exactly 2 sites (BusError/AddressError carry
`_eh_default|_eh_address_error` → flag byte `$01+$20|align`). Design the surface
(overseer default unless the porter finds a better shape: a trailing options list
`raise_error "BUS ERROR", address_error` accepting the named error-handler flags;
NOT the rejected two-arg consoleprogram form — parser.rs:1913-1920 stays). TDD
against the AS-side macro expansion via the `diag_assert_vector.rs` pattern (assemble
the real `__ErrorMessage` through the AS front-end, byte-diff the construct's
emission). Any negative probe added obeys the POSITIVE-CONTROL RULE
(campaign-port-loop.md, t24 bar).

## HAZARD INPUTS PRE-NAMED (C2 lens)

- The align-pad parity rule: each stub's flag byte ORs `$80` + emits a pad when the
  flag lands even (diag.rs:538-541 implements it) — stub-by-stub parity depends on
  preceding string lengths; the byte gate catches any miss, but the porter should
  eyeball the first divergence rather than guess.
- The blob is OPAQUE VENDORED BINARY (MD Debugger v2.6, (c) Vladikcomper) —
  transliterate `dc.l` verbatim, CARRY THE ATTRIBUTION HEADER into the .emp, change
  NOTHING inside it (step-2/step-5 have no license to touch blob bytes; step-5's
  only lawful targets are the stub table and sound_debug's copy loops).
  **VOLENCE RULED 2026-07-28: kept-for-now, replaced post-twin-retirement** — an
  own `.emp`-native diagnostics runtime is a POST-TWIN-RETIREMENT bucket row (see
  ledger duties), NOT t25 work.
- `EndOfRom` adjacency: the "DO NOT put any data from now on" warning
  (error_handler.asm tail) binds the .emp module too — the region MUST stay the
  final emission; the convsym appendix rides after it.
- sound_debug stops/starts the Z80 (stopZ80/startZ80 macro pair) — mirror-shape twin
  parity must cover the paired macro expansion; C3 is INACTIVE for error_handler
  (opaque data + cold error path) but ACTIVE for sound_debug's Z80 bus window if the
  mirror lane proceeds.

## EXPECTED BYTE MOVEMENT: ZERO

Stubs are construct-emitted byte-locked; the blob is data; sound_debug exists only at
the mirror shape; all cross-references are abs.l. Canonical CRCs
(plain `c51342d0`/421041 · debug `992d9e7d`/429102) and EndOfRom (0x5DB60/0x5F65A)
are expected UNCHANGED at every commit. Any nonzero delta is a STOP-and-report, not
an absorb-and-re-pin.

## LOOP + PANEL + BARS

Full loop `0 → 1 → 2 → (3→4→5)* → panel → 6`, LEAN posture. Step 2 on the stub
table/sound_debug only (blob untouchable). PANEL RULED: **A1 + B1 + C2; C1 INACTIVE
(cold error path — recorded, not run empty), C3 CONDITIONAL** (runs iff the
sound_debug mirror lane ships — Z80 bus window). Dry claim requires the panel per
the standing rule. Bars carried: strict baseline **2604/0** paired with AEON_DIR at
the BRANCH tree; branch worktrees `port-tranche25` both repos; fresh aeon worktree
needs the editor-dir rsync before first build; shell cwd resets every Bash — cd into
the worktree every time, never chain two repos' git in one compound. Checkpoints:
**(a) STOP after steps 0-2** for overseer countersign (own rebuild, own strict),
(b) after each pass wave, (c) merge gate. Ledger duties at close: the
POST-TWIN-RETIREMENT debugger row (§1), **a second POST-TWIN-RETIREMENT row
(Volence-prompted 2026-07-28): replace the vendored MD-Debugger blob with an own
`.emp`-native diagnostics runtime** — design seeds: Sigil emits the symbol table
natively at link (kills the convsym appendix + its byte-gate allowlist ceremony),
the diag construct sheds the third-party format mirror (frees kill row 21's pins),
the handler is sized to the USED surface (register dump + symbolized PC +
backtrace; Console/KDebug have zero corpus sites — Oracle MCP covers live
debugging; the on-screen handler's unique value is crashes on real hardware /
foreign emulators) — kill row 42 killed, row 21 condition re-cited (twin
retirement), §23 index annotated, campaign-gap-ledger sweep with the
row-1034/1085 lesson in mind (READ the grep output, name every hit).
