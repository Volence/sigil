# 2026-07-29 — t31 brief: game-side G3 — test_parent (the struct-overlay object)

Status: **DISPATCH BRIEF** (overseer: Fable; porter: Opus subagent, direct-dispatch).
Target = the census's G3 tranche (read its G3 rows as STATUS-AMENDED through t30, the t30
close packet's descriptor census, and the t29/t30 close packets' standing pattern).

## 0. Bars (overseer-verified at dispatch)

- Masters: aeon **`b381a83`** / sigil **`72baa7b`**, origin==local, clean.
- Canonical: plain **`c51342d0`/421041** · debug **`992d9e7d`/429102**. Strict baseline
  **2757/0 (1 ignored)**.
- Branches `port-tranche31` BOTH repos, worktrees `.worktrees/port-tranche31`; editor-dir
  rsync; one shape per invocation; cd-every-call; explicit paths; no `git add -u`; never
  chain two repos' git. CANONICAL-BYTES tranche: step-1 delta ZERO; step-2 flips ride the
  wave discipline. Checkpoints (a)/(b)/(c) standard; loop text `campaign-port-loop.md`;
  positive controls; kill rows same-commit; comments describe function. Valve standing.
- **MERGE-QUEUE NOTE:** t32 (the psg port) runs in parallel on `port-tranche32` branches;
  t31 merges FIRST. Different files — no expected collision; conflicts = STOP.

## 1. Scope (FIRM: one file + one ruled engine hoist)

- `games/sonic4/objects/test_parent.asm` → `test_parent.emp`. First game-side STRUCT
  overlay twins (`TParentV`/`TOrbitChildV`, the row-25 class), child dispatch,
  GetSineCosine. `test_player` is NOT in scope (P1's arc).
- **THE SpawnDesc HOIST IS PRE-RULED** (t30's descriptor census): hoist ONE shared 4-byte
  `SpawnDesc { code: ObjRoutine :w, x_off: i8, y_off: i8 }` (+ the separate `dc.w 0`
  terminator convention) to **children.emp** — the format's owner (the ObjDef-in-sst.emp
  precedent). test_parent's 3-entry `.child_desc` (test_parent.asm:144) is the multi-entry
  consumer; the two G2 emitters' fused 6-byte `EffectSpawn1` structs are REPLACED by
  SpawnDesc + their trailing term field in the same commit (byte-identical; the G2 byte
  gates prove it). This is a byte-neutral ENGINE-file data-decl touch — children.emp gains
  the struct, no code changes; the strict suite + G2/G3 gates are the proof.
- The census's PORTER-VERIFY on the overlay-collision assert → `ensure` is yours.

## 2. Known inputs

- The `Overlay.field(aN)` spelling now has its 2nd consumer — per the t29 ruling it JOINS
  the step-2 idiom list this tranche (one commit in campaign-port-loop.md, the standing
  feed-forward shape).
- The callee-preserves oracle is live — if test_parent's Main writes a0 around preserving
  calls, declare the honest contract; the oracle proves it (consumer count grows from 1 —
  note it for the census row).
- GetSineCosine: check its .emp status and contract before assuming (tree wins).
- vram_bytes: if test_parent consumes it, row 63's hoist TRIPS (2nd consumer) — in scope,
  byte-neutral, test_animated updated same commit.

## 3. Template + panel

t29/t30 standing pattern (per-file gate `SIGIL_EMP_TEST_PARENT`, test file
`test_g3_objects_port.rs`, mixed fn `mixed_tranche31`; windowed both shapes + whole-ROM +
t24 controls; regions derived with anchors stated — remember t30's anchor-error lesson:
the byte gate adjudicates, listings inform). Panel **A1 + B1 + C2**; C1 flagged-call with
named sites (child dispatch + sine lookup are per-frame); C3 inactive. Dry by panel.

## 4. Duties

Kill rows (gate arms; overlay twins — truth = the .asm twin until Spec 5; SpawnDesc's AS
twins across THREE files now — one row with the full member list). Ledger per pass;
item-13 counts updated. Close packet + census STATUS AMENDMENT (G3 ported; oracle
consumer count). After t31: the P1 player keystone arc (its own mini-arc per the census).
