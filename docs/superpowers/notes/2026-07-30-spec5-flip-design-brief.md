# 2026-07-30 — the SPEC-5 FLIP design brief (DESIGN GATE ONLY — the terminal event's plan)

Status: **DISPATCH BRIEF — DESIGN-ONLY** (overseer: Fable; porter: Opus subagent).
The flip is the campaign's terminal event: the manifest goes sigil-native, asl
retires, and every remaining `.asm` twin deletes. This parcel produces the DESIGN —
no implementation, no deletion. Sigil master = THIS brief's commit; aeon master
**`409b8ba`**.

## 0. Bars

- Sigil branch `spec5-flip-design`, worktree `.worktrees/spec5-flip-design`; aeon
  strictly READ-ONLY. Verify the baseline first: both shapes exit-0 (SIGIL_EMIT
  required), artifacts `eff2396f`/413577 · `1e9097bc`/421579, PRIMARY assembled-ROM
  `e5765873`/`dab4f06c`, strict **2904/0 (1 ignored)**.
- Deliverable = ONE committed design note + STOP. Standard rules.

## 1. The design questions (the seam-2 close packet's FLIP INPUT SET is the scope)

1. **The manifest**: main.asm + the 4 config files become the sigil-native build
   description. Design the form (a `.emp` manifest? the sigil.map.toml grown? —
   argue from what exists), the include/order/org semantics sigil must own, the
   gate-collapse mechanics (every `ifndef SIGIL_EMP_*` arm resolves to its `.emp`
   include; the else-org arms + the `phase 08000h` bracket + soundBankHead die),
   and what replaces build.sh's asl+p2bin+fixheader pipeline.
2. **THE ORACLE-MODEL SHIFT (the hard question)**: today ~60 test files compare
   sigil output against the asl-built reference ROM; post-flip there is no asl.
   Design the verification model: the frozen-provenance CRCs + the strict suite +
   what each gate class becomes (windowed region gates → ? · whole-ROM gates → ? ·
   the DSM mixed tranches → ? · the t24 controls → ?). The independent-witness
   question answered honestly: what replaces asl-vs-sigil disagreement detection
   (candidates: the frozen reference ROMs as tracked goldens; emulator A/B; the
   strict suite's internal redundancy) — and what protection is genuinely LOST,
   stated plainly for Volence.
3. **The retirement enumeration**: EVERY remaining `.asm`/twin/scaffold with its
   disposition — the 68k gate-off body twins (kill row 5's survivors + rows
   79/84/85/86/88/89 etc. — enumerate from the kill-list, don't estimate), the
   internal-gate keystone files (72-class), sound_api.asm (the deferred parcel —
   rides the flip or precedes it, rule with reason), movingtrucks_pitchtable.asm,
   row-92's vestigial scaffolding, the row-54 constant mirrors, the BODY_STUB
   defines (rows 91+), the drift guards (which die with their AS readers, which
   convert), the surviving generators (ojz_entity_gen + the level-tree
   reproducibility row — in the flip or its own precursor parcel, rule), asl/asw +
   the win32 tools + verify_emit_bin's asl-era preflights, and build.sh's final
   shape.
4. **Sequencing + safety**: the flip cannot be one commit — design the staged
   order (each stage dual-proven where a dual state exists; the point of no return
   named explicitly — the commit after which asl can no longer build the ROM), the
   rollback story per stage, and which stages are byte-identity-provable vs which
   need the frozen-golden model. Estimate what the strict count becomes.
5. **The post-flip arc handoff**: what the optimization sweep + language-ask round
   + capstone sweep inherit (the ledger's asks with counts; the §17 backlog; the
   oracle-A/B items G9/parallax; the B5/B6/B7 sweeps; the census-refresh duty).

## 2. STOP

Commit the design note, report the five answers + open questions, STOP. The
overseer reviews; the design and its execution plan go in the MORNING REPORT for
Volence before the point-of-no-return stage executes (the deletion arc is ratified;
the no-return commit still gets his eyes on the plan).
