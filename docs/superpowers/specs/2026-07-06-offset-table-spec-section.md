# Proposed spec section — `offsets` (for `empyrean/docs/SIGIL_SPEC2_LANGUAGE.md`)

> **Not committed to empyrean by the implementation branch.** `empyrean` is a separate
> repo with pending WIP in `SIGIL_SPEC2_LANGUAGE.md`, and spec authorship is Fable's role.
> This is the drafted §4.7 to lift into that file (after §4.6, before §5), in the doc's voice.
> Behavior below is verified against the implementation (branch `offset-table`) + the AS
> byte-diff cross-check.

---

### 4.7 Offset tables (`offsets`)

`offsets` declares a **bidirectional** self-relative offset table — the #1 data idiom in the
Sonic trees (`dc.w Target-Base`: mappings, DPLC, art-pointer and object/sound index tables):

```
offsets AttackTable {
    Idle:   Attack_Idle,       // emits  dc.w Attack_Idle   - AttackTable
    Windup: Attack_Windup,     //        dc.w Attack_Windup - AttackTable
    Fire:   Attack_Fire,       //        dc.w Attack_Fire   - AttackTable
}
```

**Forward** — the block emits one big-endian word per member, `member.target − AttackTable`,
in declaration order; the block's own name is the base label, defined at the table's first
byte. Each offset is checked to fit a **signed word** (`-$8000..=$7FFF`); an offset outside that
range is a **compile error** (the totality tenet — this replaces the hand-written
`if (End-Base)>$7FFF` guards), never a silently-wrapped word. Offset tables are 68k
big-endian only; an `offsets` block in a Z80 section is the `[offsets.non-68k]` error. Per §4.3
the compiler inserts **no** alignment padding — unlike AS `dc.w`, an `offsets` block at an odd
address is not silently word-aligned (offset tables are word-aligned in practice; an odd base is
the author's to fix).

**Reverse** — the same declaration introduces the comptime constants `AttackTable.Idle == 0`,
`AttackTable.Windup == 1`, `AttackTable.Fire == 2` (the member's 0-based index) and
`AttackTable.count == 3`. These are **plain comptime integers**, usable wherever a number is
(a `dc.b`, an immediate, arithmetic, a struct/bitfield field, a refinement bound) — they replace
the hand-synced `ObjID_x = $n` / `SndID_x = $n` constant blocks (778 of them in S2). Because
call-sites reference entries **by name**, inserting or reordering a row can never silently
renumber a downstream id. `count` is a **reserved** member name (a real member named `count` is a
compile error, since `.count` names the entry count). A duplicate member name is a compile error.

Ordinals are deliberately named integer constants, **not** a distinct enum type and **not** a
coercing type: there is no implicit enum→int coercion (a Haskell-flavored language avoids silent
coercions), the win is exactly the integer constants they replace, and the type-safety /
exhaustive-`match` benefit of a distinct id type only pays off for **state dispatch**, which is a
separate, encoding-agnostic construct (see the scope note). Promoting `offsets` ids to a distinct
`newtype` later is byte-neutral (newtypes are erasing, §4.1).

A member's `target` is a label reference resolved by name at link time (a `data`/`proc` label, or
another table's base). A target that is positively a `const` (`const F = ...; offsets M { A: F }`)
or a non-label expression is rejected early with a clear diagnostic; a genuinely-undefined target
fails loudly at link.

**Scope.** `offsets` is the DATA offset table only. Sonic's self-relative word-offset encoding
does not generalize to **state dispatch** (Vectorman uses raw absolute code pointers, Ristar
pre-shifted ×4 indices, Treasure word-index tables); dispatch is a separate, encoding-agnostic
construct. **Deferred** (each a later item): an explicit `base:`/`start:` override, `dc.l`
(long) offsets, Z80 offset tables, cross-module/multi-segment targets, and inline-target blocks
(frame bodies co-located inside the `offsets{}` block).
