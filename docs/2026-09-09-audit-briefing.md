# Audit briefing for the sigil lane — 2026-09-09 evening

**From:** the owner's audit session in aeon, at the owner's request: *"Fix the cards. Answer any you
can. Send info on any cleanups or things the current running agents should do from your audit."*
Every claim cites where it was read. No cargo was run in this repo (shared target dirs); the code
findings come from a read-only reviewer sampling 14 code commits since 2026-09-06.

## 1. Owner cards — what changed in `docs/decisions.jsonl` and what the lane owes

All four blocking rows failed the Dominion contract (`options[]` without `key`/`name`, `recommend`
as a string), so the console rendered them as unanswerable. They are normalised; three are answered.
**Drop d-29, d-27 and d-24 from `blockedOnOwner` in the next board write.**

| card | now | note |
|---|---|---|
| d-29 | answered `land` (by hub, delegated) | Sonic 2 errors 5,136 → 149, none added, CRCs unchanged; land AS-NAMELESS-LABELS-RC1 through the normal ritual |
| d-27 | answered `one_name_each` (by hub, delegated) | `z80` and `m68000`; refuse `m68k` by name with the fix-it |
| d-24 | answered `as-built` (by **owner**, reconciled) | `docs/OVERSEER.md:101` records the owner answering on 2026-09-04 and merge `756c7efd` landed it. The row and the board still listed it open. **The record contradicts itself in four places**: OVERSEER.md:61 says nameless-labels lands on d-24, the queue row says d-29, `lens-findings.jsonl:75` says d-22. One reconciling edit is owed |
| d-23 | schema fixed, **left open** | compatibility policy (two of three options refuse programs AS accepts). The lane's `split` recommendation is sound |

Also fix the non-blocking rows with the same defect so the ledger parses clean: d-19, d-22, d-26
(`options[*]` without `key`; `recommend` strings).

## 2. Cleanups and risks, in priority order

1. **~13 GB of stray build dirs in the main checkout**: `.target-atomic`, `.target-s1dir`,
   `.target-s1expr` (1.8 GB each), `.target-land` (4 GB), `.target-overseer` (891 MB), five more at
   ~143 MB, plus `.landing-*.log` files — all untracked, beside a 35 GB `target/`. Delete what no
   live lane owns.
2. **`flag_arms` help gate is a text heuristic** (`1d32402e`, `crates/sigil-cli/src/main.rs:504-523`):
   it reads only lines that start with `"` and contain `=>`, so a multi-line `"--a"\n| "--b" =>` arm is
   invisible, and `fn_body` ends at the first `\n}\n`. The gate that "makes undocumented flags
   impossible" holds for today's 25 single-line arms only.
3. **Z80 `lower_abs16` fixup kind changed globally** (`bbaf6bcd`, `backend-z80/lib.rs:69`,
   `BankPtr16Le` → range-checked `Value16Le`). Correct per asl, but the commit shows no four-shape ROM
   CRC re-check; confirm the sound-driver blob CRCs were re-pinned at that landing.
4. **Memo key omits `charset`** (`c3a4c4ff` HeadKey vs `eaf4ef29` threading `charset` into
   `dispatch_head_checked`, `eval.rs:3801`). Harmless today (only the keyword is memoised); make the
   invariant explicit before a token-vector memo makes it wrong.
5. **`451cb3e2` cites `.scratch/repro/control.asm` as its witness** — untracked, so the pointer cannot
   be followed. Commit the witness or restate the evidence in the message.
6. **75 asl probe fixtures under `.s1probe/` with no README** (26 added by `9aed994f`).
7. **`Entry.flags_fn`** is a shipping-struct field that exists only for a test (`main.rs:127`,
   `#[allow(dead_code)]`).
8. **AEON-REFREEZE-DEBT** would advance the reference tree by 93 commits; it is a decision, not
   housekeeping, and the board says so — keep it on the owner only if the pairing is genuinely at risk.

## 3. Quality verdict for the record

Sampled grades (14 commits): correctness A-, Rust A-, tests A, cleanliness B+, message honesty A.
Zero `unsafe` added; diagnostics read like a product. Red-first is quoted with bytes and exit codes.
Decisions: discriminating experiments run (d-28 landing, the integer-selector negative probe, the
charset third consumer); the two misses (board regenerated from a `head -60` read; an owner hold
reversed unread) were self-reported and repaired. 35% of commits since 09-06 touch code.
