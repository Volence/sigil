# 2026-07-30 — FLIP STAGE 1 brief: the dual native build (design checkpoint first)

Stage 1 of the ratified flip plan (`2026-07-30-spec5-flip-design.md` §4): stand up
the sigil-native whole-ROM build as an ADDITIVE second path while asl keeps building
everything. This is the LAST moment asl is a live independent witness on the full
program — the proof bar is the strongest the campaign will ever have, and Stage 2
(the point of no return, Volence-ruled GO) depends entirely on this stage's evidence.

## 0. Bars + rulings in force

- Baseline (overseer-verified, masters aeon `34023be` / sigil `a08ec57`): artifacts
  `eff2396f`/413577 · `1e9097bc`/421579, PRIMARY assembled-ROM `e5765873`/`dab4f06c`,
  strict **2906/0 (1 ignored)**. Builds need
  `SIGIL_EMIT=<sigil>/target/release/emit_sound_blob`.
- **Volence rulings (2026-07-30 morning, binding):** Stage 2 = GO · asl-the-binary is
  NOT kept after Stage 2 (nothing retained from the old toolchain) ·
  **`sigil-frontend-as` is PERMANENT** (the D5/Stage-4 deletion is CANCELLED —
  AS-comprehension is a feature) · **the demo game flips IN LOCKSTEP** ·
  checksum folds into `emit_rom` (OQ-4 default ruled).
- Stage-1 rule: EVERYTHING here is additive. The asl path stays the default and
  byte-identical throughout; nothing gets deleted. Rollback = revert.
- Sigil branch `flip-stage1`, worktree `.worktrees/flip-stage1`. Aeon changes only
  where the native path needs them, always keeping the asl build byte-identical
  (dual-state edits per the established gate pattern); aeon work in a seeded worktree
  (`tools/seed-worktree.sh` — the generated level tree is gitignored).
- Design checkpoint FIRST: a short committed design note (the §2 answers + the
  execution plan) + STOP for overseer countersign, then execution on endorsement.

## 1. What Stage 1 delivers (from the ratified design §4, made concrete)

1. **The native driver**: `sigil build --aeon` grows from "assemble the tree
   byte-identical at gates OFF" to the GATES-ON native path — every `SIGIL_EMP_*`
   define set, every ported `.emp` module natively lowered and placed, residual
   `.asm` (config, generated level tree, parallax data, demo game-side) consumed by
   `sigil-frontend-as`, all linked in ONE image, `emit_rom` writing the final ROM.
2. **`sigil.map.toml` grows into the placement manifest** (design §1.2/1.3): the
   section ordering + region geometry the `main.asm`/`engine.inc` gate-resume `org`s
   encode today. OQ-6 (declarative vs computed link outputs) is yours to settle in
   the design note — prefer computed link outputs wherever the resume-org exists
   only because the dual build exists (kill rows 6/58 die at Stage 2 either way).
3. **Checksum**: fold the Genesis header checksum into the sigil emit path (ruled);
   `tools/fixheader` keeps serving the asl path during the dual state.
4. **Symbols/listing**: the debug artifact is ROM + convsym deb2 appendix, and
   convsym reads an asl `.lst`. Post-flip that listing must come from sigil
   (`SIGIL_CORE_SPEC.md` D3 — AS-`.lst`-compatible listing). Stage 1 must prove the
   FULL-FILE artifact reproduces through the native path: sigil listing → convsym →
   appendix byte-identical, i.e. native `s4.debug.bin` == asl `s4.debug.bin` ==
   `1e9097bc`/421579, not just the assembled-ROM prefix.
5. **THE DEMO IN LOCKSTEP**: `sigil build` for `games/demo` (game-side `.asm` via
   `sigil-frontend-as` against the same engine sources), `demo.bin` (both shapes if
   demo has shapes) captured as a frozen golden from the asl build and matched
   natively. Demo must be provable here or Stage 2 cannot delete the engine twins.
6. **The maximal golden freeze** (design §2.3 mitigation 1): capture + commit the
   frozen golden set Stage 2 will gate on — both shapes, both games, AND the
   off-canonical Config-A/B ROMs (kill rows 55/58 geometry), with a provenance note.
   The t24 positive-control/negative-probe discipline carries over verbatim.
7. **The build entrypoint**: a non-default native invocation (env flag or script
   path) so the dual proof is one command per side; `build.sh`'s default is
   untouched until Stage 2.

## 2. Design-checkpoint questions (answer before executing)

1. Current `run_build`/`assemble_full_rom` gap analysis: what exactly is missing
   between today's gates-OFF byte-identity and the gates-ON native path (module
   registry, placement of each `SIGIL_EMP_*` region, the banked window, the
   emitted-blob BINCLUDEs vs native linking of the same `.emp` modules)?
2. The map-manifest growth plan (which sections/orderings/budget asserts move into
   `sigil.map.toml`, which resume points become computed) — OQ-6 ruled with reasons.
3. The listing/convsym path: does sigil's listing already satisfy convsym? What
   gap remains for full-file debug identity?
4. Demo: its config/gates census (does demo even have SIGIL_EMP gates today? it
   includes `engine.inc` — how do the engine gates apply to demo's build?), and the
   lockstep mechanics.
5. The off-canonical golden set: enumerate exactly which configs get frozen.
6. The proof matrix + strict-suite additions: the Stage-1 gates (native == asl ==
   golden per game × shape × config) as named tests, and what Stage 2 will
   re-comparand them to.

## 3. Proof model (the stage exit bar)

`sigil-native whole ROM == asl whole ROM == frozen golden` — both shapes × both
games × the off-canonical set, full-file INCLUDING the debug appendix, plus strict
suite green with the new gates and the asl default path still at
`eff2396f`/`1e9097bc` and PRIMARY `e5765873`/`dab4f06c` unmoved. Every claim
own-run by the overseer at the countersign.

## 4. Deliverable

Design note `2026-07-30-flip-stage1-design.md` committed on `flip-stage1` + STOP;
then staged execution on endorsement (each aeon-touching commit dual-proven), a
close checkpoint with the full proof matrix, overseer merge.
