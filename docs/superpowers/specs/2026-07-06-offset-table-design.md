# Design — Bidirectional offset-table (`offsets`) · Spec 2 · Plan 7 · backlog #3

Written 2026-07-06. Approved surface + architecture with Volence before implementation.
Supersedes the roadmap sketch in the `emp-data-table-dsl-candidates` memory for this item.

## Problem

`dc.w Target-Base` — a 16-bit **self-relative word offset** from a base label to each target —
is the #1 data idiom in every Sonic tree (**14,179** lines S3K, 4,627 S2, 866 legacy). It backs
mappings, DPLC, art-pointer, and object/sound index tables. It is the **Plan-6 blocker**: `.emp`
cannot express a symbol *difference* in data today, because `Cell::SymRef` is absolute-only — it
yields an absolute `Abs16Be`/`Abs32Be` fixup, never a difference.

S2 also uses the **inverse**: `id()` derives a named constant from a label's *index* in the table
— **778 hand-synced cross-file constants** (217 ObjID, 79 SndID, …), each requiring the author to
set three globals (`offset`/`ptrsize`/`idstart`) before the block. SCE promoted both directions to
named `offsetTable`/`offsetTableEntry` + `id` macros — independent confirmation this is a
deliberate, bidirectional abstraction.

## Scope

**In (this milestone — full bidirectional):**
- Forward: emit `dc.w target − base` words, base = the table's own start label, each range-checked
  to signed word (±$7FFF) with overflow as a **compile error** (totality).
- Reverse: the block also declares **named comptime ordinal constants** — `Table.Variant` = its
  0-based index in the table, an ordinary comptime integer; `Table.count` = entry count. (Named
  integer constants, NOT a distinct enum type — see the decision note below.)
- 68k big-endian sections only.

**Deferred (each its own future item — diagnosed, not mis-handled):**
- Optional `base:`/`start:` override (S3K occasionally bases offsets on a different label / a
  nonzero id start). Default: base = table label, ids start at 0.
- `dc.l Target-Base` long offsets (`RelLong32Be`).
- Z80 offset tables.
- Inline-target blocks (frame bodies co-located *inside* the `offsets{}` block). First cut
  references pre-existing named `data`/`proc` targets — which, because `.emp` has no bare
  `loc_XXXX` labels, is the universal shape anyway.

**Out (explicitly, per research R1):** state dispatch. Sonic's self-relative word-offset encoding
does NOT generalize to state dispatch (Vectorman = raw absolute pointers, Ristar = pre-shifted ×4
IDs, Treasure = word-index). State dispatch is a separate, encoding-agnostic construct. This item
is the DATA offset-table only.

## Surface

```emp
offsets Map_PitcherPlant {
    Idle:  frame_idle,      // emits  dc.w frame_idle  - Map_PitcherPlant
    Shoot: frame_shoot,     //        dc.w frame_shoot - Map_PitcherPlant
    Seed:  frame_seed,      //        dc.w frame_seed  - Map_PitcherPlant
}

data frame_idle  = [ ... ]  // the runs the offsets point at
data frame_shoot = [ ... ]
data frame_seed  = [ ... ]
```

- Each entry is `VariantName: target_expr`, where `target_expr` names a relocatable symbol (a
  `data`/`proc`/label — NOT a `const`, which folds early and never relocates).
- **Forward:** the block emits `count` big-endian words; word *i* = `entries[i].target − Table`.
  The table's own start label is `Table` (the block name is the base symbol).
- **Reverse:** the block also introduces the named ordinal constants `Table.Idle == 0`,
  `Table.Shoot == 1`, `Table.Seed == 2` — ordinary comptime integers usable directly wherever a
  number is expected (`dc.b Table.Seed`, immediates, indices), erasing to zero runtime cost.
  `Table.count` is a comptime integer. Referencing entries by name means inserting a row can never
  silently renumber downstream ids. Unknown `Table.Variant` and duplicate variant names are
  compile errors.

`offsets` is `pub`-able like any declaration to export the base symbol + the ordinal constants
across modules.

### Decision note: constants, not a distinct enum type (2026-07-06)

The reverse direction is deliberately **named integer constants**, not a distinct closed-enum
type. Rationale: (1) it directly replaces the 778 hand-synced `ObjID_x = $n` / `SndID_x = $n`
*integer* constants, which is the actual win; (2) a distinct enum type would need an implicit
enum→byte coercion at every `dc.b`/immediate use-site — exactly the kind of silent coercion a
Haskell-flavored language avoids (tenet: illegal states don't compile, no lossy implicits); (3)
the type-safety benefit of a distinct id type (and exhaustive `match` over it) only pays off for
**state dispatch**, which is explicitly out of scope here (research R1). Promoting `offsets` ids to
a distinct `newtype`/closed-enum — for cross-table id type-safety and exhaustive dispatch — is a
clean, byte-neutral future layer (newtypes are erasing, §4.1) and is left for the encoding-agnostic
dispatch construct that will consume these tables.

## Architecture

Five focused, independently-testable pieces.

### 1. `FixupKind::RelWord16Be` (`crates/sigil-ir/src/fixup.rs`)

A new fixed-width fixup kind. `byte_width() = 2`. Semantics: resolve `target` (an `Expr`) to a
value, interpret it as a **signed 16-bit** displacement, write big-endian. It carries the
subtraction in its existing `target: Expr` field as `Expr::Sub(Sym(target), Sym(base))` — no
two-symbol struct is introduced; `Fixup.target` is already an expression and the existing fold
machinery computes the difference.

Doc comment must distinguish it from `Abs16Be` (absolute address truncated to 16 bits, range
`[-0x8000,0x7FFF] ∪ [0xFF_8000,0xFF_FFFF]`) vs `RelWord16Be` (a signed relative offset, range
`[-0x8000,0x7FFF]`).

### 2. Link resolution (`crates/sigil-link/src/lib.rs`, `apply_fixup`)

`apply_fixup` already folds `fx.target` against the global symbol table, so `Sub(target, base)`
folds to `target_vma − base_vma` with no new fold code. Add one match arm:

```rust
FixupKind::RelWord16Be => {
    if !(-0x8000..=0x7FFF).contains(&value) {
        diags.push(diag(format!(
            "offset out of signed-word range ({value}) in section {section}"), span));
        return;
    }
    bytes[site_abs as usize..site_abs as usize + 2]
        .copy_from_slice(&(value as i16).to_be_bytes());
}
```

Fixed-width ⇒ **no** relaxation logic required (unlike #2's `RelaxAbsSym`); it flows through as a
plain `Fragment::Data` + fixup, resolved at link time exactly like `Abs16Be`.

### 3. `.emp` parse + lowering (`crates/sigil-frontend-emp`)

- **Parse** the `offsets Name { Variant: target, ... }` block into an AST item carrying the base
  name, ordered `(variant, target_expr)` entries, and span.
- **Forward lowering** (`src/lower/data.rs` neighborhood): emit a `Fragment::Data` of `2×count`
  zero bytes with one fixup per entry — `Fixup { kind: RelWord16Be, offset: i*2,
  target: Sub(Sym(entry.target), Sym(Name)) }`. Introduce a `Cell::RelOffset { base, target }`
  mirroring `Cell::SymRef`, so `stream_data` handles it alongside the existing cell kinds.
- **Reverse (ordinal constants):** register each `Name.Variant` as a named comptime integer
  constant (its 0-based ordinal) in the comptime value environment, plus `Name.count`. These are
  plain integers — no distinct enum type, no coercion machinery (see the Decision note). Duplicate
  variant names are a compile error; an unknown `Name.Variant` reference is a compile error.

The base symbol `Name` must be emitted as a label at the table's start address so the
`Sub(target, Name)` fold resolves it — i.e. the `offsets` block defines the label `Name` at the
first emitted byte.

### 4. AS byte-diff reference (`crates/sigil-frontend-as`)

The whole-branch review byte-diffs emp output against the AS front-end wherever a byte argument
exists. **First implementation task is an investigation:** does the AS front-end fold
`dc.w Target-Base` for *forward* references (frames defined after the table)? The prior code read
shows `directive_dc_w` emits a fixup only for a bare `Sym`; a `Sub` that fails to fold errors
"unresolved word expression". Determine the AS pass structure:

- **If AS resolves forward diffs** (multi-pass fold assigns addresses first): the AS
  `dc.w Frame-Base` output is a genuine independent computation of the same offsets — the strongest
  cross-check. Use it directly in the byte-diff.
- **If AS cannot fold forward diffs:** the byte-diff golden is hand-computed literal `dc.w $NNNN`
  (still validates emp's computed bytes against an external constant). Note whether teaching AS the
  same `Target-Base` support is worth a follow-up (it is the reference front-end; parity matters
  long-term, but is not required to ship this item).

This investigation gates *which* reference the tests use; it does not change pieces 1–3.

### 5. Tests

- **Byte-diff** (`crates/sigil-cli/tests/ports.rs`, via `as_reference`/`emp_candidate`): an
  `offsets` table with real forward-referenced frame `data` runs == the AS/golden `dc.w` bytes.
  Include a **negative** offset (a target defined *before* the base) to exercise two's-complement.
- **Totality:** an offset that exceeds +$7FFF is a compile **error** (emp-only assertion; AS
  silently truncates via `v as u16` — the intended, documented divergence).
- **Ordinal constants:** `Name.Variant` values are 0,1,2,…; `Name.count` correct; `Name.Variant`
  usable as a `dc.b` and as an immediate; a duplicate variant and an unknown `Name.Variant` each
  error.
- **End-to-end:** `sigil emp <file.emp>` compiles a small module using `offsets` for both a
  mapping table (forward) and an object-index id (reverse) and produces the expected bytes.

## Process

Per the standing Plan-7 process (non-negotiable):
- Isolated git worktree `sigil/.worktrees/<branch>`; subagent-driven; **TDD per task,
  commit-per-task.**
- Green gate before every commit: `cargo test --workspace` +
  `cargo clippy --workspace --all-targets -- -D warnings`.
- Two-stage reviews (spec-compliance THEN code-quality via `superpowers:code-reviewer`) on
  load-bearing tasks; whole-branch adversarial review at the end that constructs + runs
  cross-feature programs and byte-diffs against the AS reference.
- Ground semantics in the spec, but where spec and code disagree, the **code** is authoritative.
- **Milestone checkpoint with Volence before merge to master** (`--no-ff` merge + push).

## References

- Research T1-a + R1: `docs/superpowers/specs/2026-07-06-sigil-spec2-p7-language-completion-research.md`
- Handoff: `docs/superpowers/notes/2026-07-06-spec2-plan7-item3-offset-table-handoff.md`
- The #2 template (deferred-width / relocation work): `sigil-ir` `Fragment::RelaxAbsSym`,
  `sigil-link/src/relax.rs`, `sigil-frontend-emp/src/lower/code.rs`.
- Spec §4.5 typed data emission; §4.4 closed enums; conventions on relocatable labels vs `const`.
- Memory: `spec2-progress`, `emp-data-table-dsl-candidates`, `emp-language-design-principles`.
