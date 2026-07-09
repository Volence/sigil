# Port #1 — `engine/system/hblank.asm` → `hblank.emp` (the campaign opener)

Per the ratified campaign kickoff (`docs/superpowers/notes/2026-07-09-68k-campaign-kickoff.md`):
first CODE port, deliberately tiny (18 lines / 18 bytes), align-free, `__DEBUG__`-free, one
inbound cross-seam symbol. The deliverable includes the **gate-pattern writeup** for the rest
of the campaign. Discipline: sigil worktree branch `port-hblank` + aeon worktree branch
`sigil-emp-hblank`, TDD, per-task two-stage reviews, UNMERGED for Volence's checkpoint.
Code-sense standard applies ([[port-code-sense-review]]): byte-exact is the gate, not the bar;
byte-neutral readability lands with the port; anything byte/behavior-touching goes on the
"reads wrong" list for a post-port commit.

## Fact base (orchestrator recon, 2026-07-09 — all verified against live listings/probes)

- **The file** (`aeon/engine/system/hblank.asm`, included from `engine/engine.inc:92` between
  vblank and controllers): `HBlank_Dispatch` (movem.l push d0-d1/a0 → movea.l
  `(HBlank_Handler_Ptr).w`,a0 → jsr (a0) → movem.l pop → rte) + `HBlank_Null` (rts).
- **Reference bytes (both shapes, identical 18 bytes):**
  `48E7 C080 | 2078 8022 | 4E90 | 4CDF 0103 | 4E73 | 4E75`.
- **Addresses:** plain `HBlank_Dispatch=$227E`, `HBlank_Null=$228E`, region `$227E..$2290`;
  debug `$230C` / `$231C`, region `$230C..$231E`. Gate-resume org = `$2290` plain / `$231E`
  debug.
- **Cross-seam:** INBOUND — `HBlank_Handler_Ptr` is an AS-side **label** (`ram.asm:72`,
  `$FFFF8022`, same value both shapes; labels reach the shared link symbol table, so no
  `extern()` needed for an *operand* reference). OUTBOUND — `vectors.asm:36` `dc.l
  HBlank_Dispatch` (dc.l deferral, proven) and `boot.asm:185` `move.l #HBlank_Null` (imm32
  deferral, D2.27, proven).
- **Probe results** (scratchpad `hblank_probe*.emp`, compiled via `sigil emp`):
  - Everything EXCEPT movem lowers byte-exact TODAY: `movea.l HBlank_Handler_Ptr, a0` emits
    `2078 8022` (RelaxAbsSym + `asl_width_rule`, which masks to 24-bit so `$FFFF8022 →
    $FF8022 → abs.w` — correct by construction); `jsr (a0)`=4E90, `rte`=4E73, `rts`=4E75.
  - **GAP 1 — movem register lists:** `movem.l d0-d1/a0, -(sp)` fails: the `.emp` operand
    grammar has no reglist form (`d0-d1/a0` parses as arithmetic over unknown names). The ISA
    layer is COMPLETE (`Operand::RegList(u16)` + `encode_movem`, canonical mask in, direction
    from operand order, predecrement reversal encoder-side — asl-verified); the AS front-end
    has the reference reglist parser (`eval.rs lower_m68k_movem`, ~2707). Front-end-only work.
  - **GAP 2 — `sp`:** unknown everywhere (`-(sp)`, `(sp)+`, reglists); `-(a7)` works. The
    whole aeon tree spells `sp`, so every code port trips on this. General areg alias, not a
    §10 inventory change (a register spelling, tenet 3).
  - Non-blocking discovery (record in the completion note): a symbolic absolute operand whose
    target is an **equ** (not a label) reports `unresolved symbolic absolute operand` — the
    relaxation pass reads layout labels, not equ symbols. Fine for this port (label target);
    a diagnostic/mechanism gap to ledger for later ports.
- **Harness anchors:** port-test template = `crates/sigil-cli/tests/sfx_port.rs` (inline
  per-shape `map_toml`, parse → `lower_module` → `place_sections` → synthetic cross-seam
  sections → `resolve_layout` → `link` → `check_link_asserts`, byte-diff vs reference ROM
  slice); full-tree mixed gate = `crates/sigil-harness/tests/mixed_dac_rom.rs` +
  `assemble_mixed_*_as_side` helpers in `crates/sigil-harness/src/lib.rs`; negative-probe
  template = `crates/sigil-cli/tests/sfx_negative_probes.rs`. Reference ROMs: `AEON_DIR`
  s4.bin 451198 B / s4.debug.bin 458982 B (match `golden/PROVENANCE.md` pins at aeon
  `a103e46`).
- **Risk to verify in T3 (not assumed):** a `pub proc`'s name must surface as the BARE link
  symbol (`HBlank_Dispatch`, not owner-qualified) for the AS-side consumers. Data items are
  proven bare (Plan 6 T4); procs are not yet — the synthetic-consumer test settles it.

## Tasks

### T1 (sigil): `sp` alias + `movem` register lists in `sigil-frontend-emp`
TDD; mirror the AS front-end's reglist semantics (canonical mask emitted; encoder owns
direction/reversal). Scope:
- `sp` accepted as `a7` wherever an address register parses (EAs: `(sp)`, `(sp)+`, `-(sp)`,
  `d(sp)`; reglists; plain register operand). Byte-identity with the `a7` spelling.
- Reglist operand for `movem` (both directions): ranges `d0-d2`, unions `/`, single register,
  mixed `d`/`a`, `sp` inside lists. Emitted as canonical `Operand::RegList(mask)`.
- Byte-parity tests vs the AS reference (ports-harness `as_reference` style or direct
  encode expectations): `movem.l d0-d1/a0,-(sp)` → `48E7 C080`; `movem.l (sp)+,d0-d1/a0` →
  `4CDF 0103`; a `.w` form; a single-register form; a wide mixed list. Error cases: `movem.b`,
  malformed/empty list, descending range, reglist on a non-movem mnemonic — clean diagnostics,
  matching AS behavior where AS accepts/refuses.
- Commit on green + clippy clean.

### T2 (aeon): `engine/system/hblank.emp` + the `SIGIL_EMP_HBLANK` gate
- Write `engine/system/hblank.emp`: `module engine.hblank`, `@as_compat`, one
  `section hblank (cpu: m68000)` (NO vma — the map pins it per shape), `pub proc
  HBlank_Dispatch` + `pub proc HBlank_Null`, instruction lines verbatim from the .asm
  (keep the `sp` spelling), comments carried per function-not-history.
- Gate at `engine/engine.inc:92`, SFX-spelling:
  `ifndef SIGIL_EMP_HBLANK / include "engine/system/hblank.asm" / else / ifdef __DEBUG__ org
  $231E / else org $2290 / endif / endif` + a comment noting the org values are
  sonic4-shape-specific reference addresses (see PROVENANCE; re-pin on re-baseline) and that
  demo/other games must never define the gate.
- Verify gate-off byte-neutrality: `./build.sh` (+ debug shape + demo) in the aeon worktree →
  s4.bin/s4.debug.bin match the PROVENANCE sizes+hashes, demo.bin identical to a pre-change
  build. Commit.

### T3 (sigil): the byte gates + negative probes
- `crates/sigil-cli/tests/hblank_port.rs` (sfx_port sibling): per-shape inline map with region
  `hblank` (base `$227E`/`$230C`, size `$12`); compiles the REAL
  `AEON_DIR/engine/system/hblank.emp`; synthetic sections: the `ram_standin` label
  `HBlank_Handler_Ptr @ $FFFF8022` (both shapes) + a synthetic AS consumer (`dc.l
  HBlank_Dispatch` and a `move.l #HBlank_Null,…` imm32) proving the outbound labels resolve
  BARE cross-seam; byte-diff the placed section vs `s4.bin[0x227E..0x2290]` /
  `s4.debug.bin[0x230C..0x231E]`.
- Full-tree mixed gate: `assemble_mixed_hblank_as_side` (defines DAC+MT+SFX+HBLANK — the
  cumulative gate) in `sigil-harness/src/lib.rs` + a `mixed_dac_rom.rs`-style test proving the
  FULL mixed ROM byte-identical to both reference ROMs.
- Negative probes (`hblank_negative_probes.rs`): corrupt one instruction in a doctored copy →
  the diff fires (genuineness); compile standalone with the ram stand-in ABSENT → the loud
  missing-symbol diagnostic (Item-C wording names `HBlank_Handler_Ptr`); wrong-base map → the
  bytes move (placement is real, not an echo).
- Commit on green; strict-gate run (`SIGIL_STRICT_GATE=1 AEON_DIR=<aeon worktree>`).

### T4: whole-branch review + code-sense pass + writeup + checkpoint packet
- Two-prong review (spec/campaign compliance + code quality) of both branches.
- Code-sense pass on `hblank.emp` (the [[port-code-sense-review]] standard); byte-neutral
  improvements land now, anything else goes on the "reads wrong" list.
- The **gate-pattern writeup** (the kickoff deliverable): the port loop recipe for campaign
  files, in the completion note.
- Checkpoint packet: `docs/superpowers/notes/2026-07-09-port1-hblank-complete.md` (asks:
  merge both branches `--no-ff`; carry-forwards; the equ-operand discovery).

## Decisions (orchestrator, recorded)
- D-P1H.1: `sp` is a general operand-layer alias for `a7` (not movem-scoped) — every code
  port needs it; byte-identical by construction; not a §10 concept.
- D-P1H.2: reglist parsing is mnemonic-directed (movem only), mirroring the AS front-end —
  no general reglist expression form leaks into the operand grammar.
- D-P1H.3: the `.emp` spells the operand `(HBlank_Handler_Ptr).w`-equivalent as the bare
  symbolic operand (width by `asl_width_rule` at link = abs.w, proven exact) — explicit
  operand-width syntax stays deferred (Plan-7 #2 scope note), and `@as_compat` retains only
  BRANCH size pins today. If the byte gate ever disagrees, that reopens the deferral; it
  cannot here (rule masks to 24-bit).
