# Sigil M1.D — Full-ROM byte-exactness — Handoff

**Status:** SUPERSEDED as the M1.D work plan by
`2026-07-04-sigil-m1d-full-rom-spec.md` (audit-revised: new byte-affecting
`padding`-across-restore finding, the stale-fold risk upgraded to a certainty with a fix
direction, `cmpm` front-end gap, spec backports now in scope). This doc remains valid as
the M1.C landing record. Original status: M1.C (AS 68k front-end) essentially COMPLETE and
merged. M1.D is the remaining path to `sha256(sigil_s4.bin) == sha256(ref_s4.bin)`.

## Where M1.C landed
The front-end (`sigil-frontend-as`) assembles the ENTIRE real Aeon `games/sonic4/main.asm`
include tree — all 68k instruction lowering (incl. Motorola `disp(An)`, movem/movep, all EA
except 6 deferred sites), directives, macros, `set`/`:=`, `sin`/`int` fold, `org` back-patch,
BINCLUDE, the full debug surface, AND the deep phase/dephase **physical/LMA-continuity** model
(continuous physical counter + phase-as-displacement; `save`/`restore` preserve cpu/padding/
supmode only — asl-verified). The full-build recon went 1538 → ~19 diagnostics; the MovingTrucks
bank-boundary `fatal` is cleared.

**Byte-exact gates GREEN** (`SIGIL_STRICT_GATE=1 AEON_DIR=…`): `m0_acceptance` (Z80 regions),
`m1b_gate` (checksum `0x18E` vs `s4.bin`, multi-section link, emit_listing), `m1c_vector_table`
(main.asm include tree + vector table = `s4.bin[0:256]`). 139 asl-snippet goldens tool-authentic.

## Recon + emit tooling (re-run to measure)
- `cargo run -p sigil-harness --example m1c_full` (with `AEON_DIR`) — assembles the full
  `main.asm`, buckets ALL diagnostics into gap classes (`FILTER=<substr>` dumps specifics).
- `crates/sigil-harness/examples/m1c_rom.rs` — assemble → resolve_layout → link → emit_rom vs
  `s4.bin` (committed; blocked on assembly reaching 0 diagnostics).

## M1.D remaining work, in order
1. **String-valued `set` symbols + the `__FSTRING` machinery** (the immediate assembly blocker).
   `error_handler.asm`'s `__ErrorMessage` macros are NOT `__DEBUG__`-guarded, so the non-debug
   ROM runs `__FSTRING_GenerateArgumentsCode`, which stores STRINGS in symbols (`.__str: set
   "..."`) and scans them with `substr`/`strstr`/switch-on-string-type. sigil's `SymbolValue` is
   integer-only. Extend it to `Int | Str` + expression handling + the scan loop. asl supports it
   and builds `s4.bin` fine. (Source `debugger.asm:647-659`, `error_handler.asm:31-65`.)
2. **The 6 deferred m68k EA sites** (T5b out-of-scope forms surfaced by the recon) — small.
3. **Whole-ROM link/emit path — UNTESTED on real source.** Once assembly reaches 0 diagnostics,
   `m1c_rom` exercises resolve_layout/link/emit_rom over the full section set for the first time.
   Expect second-order issues here: whole-image branch-width fixpoint, `sec0`/section-name
   collisions, the `resolve_layout` Org+JmpJsrSym guard on the object-code bank, and the
   **KNOWN DEEP RISK** — the linker picks jmp/jsr abs.w/abs.l width AFTER the front-end folds
   label values, so a folded label positioned after a width-grown jmp/jsr could be stale (linker
   shifts syms + fixups but not already-folded bytes). This is where it would finally bite.
4. **Full `sha256` match for `__DEBUG__` on AND off** (A2), then **delete the ~42-symbol stub
   table** (A+C interlock — in the full build the front-end defines everything, so the stubs
   should already be unnecessary; the recon used ZERO stubs).

## Known pre-existing issue (independent, not M1.C-caused)
The `#[ignore]` live gate `harness_assembles_regions_a_and_b_together` is broken independently of
M1.C: `harness_root.asm` omits `padding off` (DacSample struct sizes 10 vs 9) and its golden
blobs are stale (region A drifted ~24 bytes vs current aeon source). Needs a deliberate `regen`
+ one line in `harness_root.asm`. Proven independent (baseline reproduces the divergence).

## Process note
M1.C ran as ~20 subagent-driven tasks (spec→plan→implement→two-stage-review) + the bounded
vector-table milestone + the full-build recon + the deep phase/dephase fix. The review loop +
"gate every byte against real asl (`gen_snippet_vectors`)" caught 10+ real byte-exactness bugs
and 4 verified spec corrections (comparisons 0/1 not 0/−1; `int()`=floor; the D5 `strstr` bug
doesn't exist in asl 1.42; `disp(An)` — the dominant real syntax — vs the AS `(d,An)` form the
synthetic snippets used). The full-build recon against REAL source was decisive: it found the
`disp(An)` miss (1100 sites) that a green synthetic suite hid.
