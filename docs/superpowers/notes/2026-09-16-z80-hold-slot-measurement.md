# The Z80 hold interleave slot: what the artifact actually requires

2026-09-16. A measurement note, not a design. Nothing in sigil was changed to write it,
and nothing was built (this parcel is read-only by instruction: a `cargo` invocation in
this checkout relinks the shared `target/release/sigil` another lane may be using).

## Revisions read

Every claim below is anchored to a published revision. A sentence that is true only of
somebody's working tree is indistinguishable, later, from a false one.

| repo | ref | revision | ref date |
|---|---|---|---|
| sigil | `origin/master` (= this worktree's base) | `26de6eb734638c0537946f98a929e1bda6b8a525` | 2026-09-16 06:53:01 -0400 |
| aeon | `origin/master` | `3794c99bfc8ce0e612df2285576ad274880630a6` | 2026-09-16 06:43:41 -0400 |
| empyrean | `origin/main` | `cabaa0d826cd9c43f44b3326ec73db25ca6af12e` | 2026-09-16 06:45:28 -0400 |

aeon and empyrean were read through `git show` / `git grep` at the ref, never through a
working tree. sigil paths are read in this worktree, whose base equals sigil `origin/master`
(verified by `rev-parse` on both).

---

## Q1. What `with z80_stopped` admits today, and where that is decided

### The accepted syntax

Parsed at `crates/sigil-frontend-emp/src/parser.rs:3115-3134` (`asm_with`), reached from
the statement dispatcher at `parser.rs:2754`:

```rust
// parser.rs:2754
if self.at_kw("with") && matches!(self.peek2(), Tok::Ident(_)) {
    return Some(self.asm_with(splices_allowed));
}
```

```rust
// parser.rs:3115-3134
fn asm_with(&mut self, splices_allowed: bool) -> AsmStmt {
    let start = self.span();
    self.bump(); // `with`
    let ctx = self.expect_ident("context name");
    let cond = if self.eat_kw("if") { Some(self.expr_no_struct_lit()) } else { None };
    let span = start.merge(self.prev_span());
    ...
    self.expect(&Tok::LBrace, "`{`");
    let body = self.asm_body(splices_allowed);
    self.expect(&Tok::RBrace, "`}` to close the `with` bracket");
    ...
    AsmStmt::With { ctx, cond, body, span }
}
```

So the entire accepted surface is `with <ident> [if <comptime expr>] { <asm statements> }`.
There is exactly one hole in it, the body. `with` is a CONTEXTUAL opener (it fires only on
`with` followed by an identifier), not a reserved statement keyword; that matters for Q5.

The context declaration is parsed at `parser.rs:2150-2225` (`context_decl`). Its body admits
four barewords and nothing else: `acquire = <expr>`, `release = <expr>`, `released_by_rte`,
`granted` (`parser.rs:2162-2188`). The unknown-field arm at `parser.rs:2180-2187` diagnoses
anything else, so today the acquire is ONE expression and the compiler has no name for any
part of it.

The AST is `ast::AsmStmt::With { ctx, cond, body, span }` (`ast.rs:2269-2278`) and
`ast::ContextKind::Acquired { acquire: Expr, release: ReleaseSpec }` (`ast.rs:192-197`).

### What is emitted, and where

Lowered at `crates/sigil-frontend-emp/src/eval/asm.rs:586-783` (`lower_with`). The emission
core is 15 lines:

```rust
// eval/asm.rs:666-701
buf.push(CodeItem::ContextMark { ctx: ..., kind: ContextMarkKind::Enter, span, released_by_rte: rte });
let acq_start = buf.items.len();
let acq_ok = self.splice_context_code(&acquire, ctx, ContextPhase::Acquire, span, buf, env);
let acq_end = buf.items.len();
buf.push(CodeItem::ContextMark { ctx: ..., kind: ContextMarkKind::AcquireEnd, ... });
self.lower_with_body(body, scope, buf, env);
buf.push(CodeItem::ContextMark { ctx: ..., kind: ContextMarkKind::BodyEnd, ... });
let rel_start = buf.items.len();
let rel_ok = match &release {
    Some(release) => self.splice_context_code(release, ctx, ContextPhase::Release, span, buf, env),
    None => true,
};
let rel_end = buf.items.len();
buf.push(CodeItem::ContextMark { ctx: ..., kind: ContextMarkKind::Exit, ... });
```

**Finding Q1-a: there is no grant-spin splice site in sigil.** The brief's question "where
is the grant-spin spliced" has the answer "nowhere, as a distinct thing." `splice_context_code`
(`eval/asm.rs:811-846`) evaluates the acquire expression to one `Value::Code`, re-authors
every `User` item in it as `ItemAuthor::Context { name, phase: Acquire }`
(`value.rs:391-399`), and extends the buffer. The bus request, the `.wait_z80` label, the
`btst` and the `bne` are four indistinguishable items inside that single `Code` value. Sigil
does not know which of them is "the request" and which is "the spin"; only aeon's
`engine/z80_bus.emp:144-149` knows, and it knows it in a comment.

That is the whole mechanical reason boot cannot be expressed today, and it is a smaller
reason than "the bracket cannot take that shape": the bracket has no seam, not a wrong seam.

### What the three proofs key off

All three are implemented in `crates/sigil-frontend-emp/src/context.rs`, and all three read
the compiler's own `ContextMark` items, never the source and never a name.

Regions are recovered from the ITEM STREAM at `context.rs:243-291` (`regions_of`), giving
`Region { ctx, enter, acquire_end, body_end, exit, span, released_by_rte }`
(`context.rs:188-205`). The three index predicates:

```rust
// context.rs:209-231
pub fn contains(&self, i: usize) -> bool { i > self.enter && i < self.exit }
pub fn in_acquire(&self, i: usize) -> bool { i > self.enter && i < self.acquire_end }
fn in_body(&self, i: usize) -> bool { i > self.enter && i < self.body_end }
```

**`[context.escape]`** keys off an out-edge of the acquire+body range that does not land back
inside the region, `context.rs:393-424`:

```rust
Edge::Follow(succ) if !region.contains(succ) => {
    fire(if region.released_by_rte { ContextFiringKind::RteUndischarged }
         else { ContextFiringKind::Escape }, span_at(items, idx, region));
}
...
Edge::Return | Edge::FallOff | Edge::TailOut | Edge::BranchOut => {
    fire(if region.released_by_rte { ... } else { ContextFiringKind::Escape }, ...);
}
```

The walk it ranges over is seeded at the region's first instruction and gated by
`region.contains` (`context.rs:340-364`). The transfer function is the identity
(`&|st, _, _| st`), which the module header states explicitly: this is reachability wearing
the shared lattice's plumbing, not a dataflow.

**`[context.entry-skip]`** keys off the TARGET SYMBOL of transfers outside the region, plus
any exported label inside it, `context.rs:429-456`:

```rust
CodeItem::Instr { mnemonic, ops, span, .. } if !region.contains(idx) => {
    if !is_transfer_mnemonic(mnemonic.as_str(), cpu) { continue; }
    let target = branch_target(ops).and_then(|t| cfg.label_index(t));
    if target.is_some_and(|t| region.contains(t) && t != entry) {
        fire(ContextFiringKind::EntrySkip, *span);
    }
}
CodeItem::Label { export: true, name, span }
    if region.contains(idx) && cfg.label_index(name) != Some(entry) =>
{ fire(ContextFiringKind::EntrySkip, *span); }
```

**`[context.reacquire]`** has TWO implementations, which matters for Q2. The nesting form is
in `regions_of` (`context.rs:252-262`): an `Enter` mark for a context already open fires. The
BRANCH form is in `check_regions` (`context.rs:374-378`):

```rust
Edge::Follow(succ)
    if region.in_acquire(succ) && !region.in_acquire(idx) =>
{ fire(ContextFiringKind::Reacquire, span_at(items, idx, region)); }
```

This is the only one of the three whose behaviour depends on where `AcquireEnd` falls. The
other two read `contains` alone.

### Comment/code agreement

I looked for a comment that the code contradicts. I found one place where the wording is
looser than the code, and it is worth naming because it is the exact seam this design
touches. `context.rs:218-220` says of `in_acquire`: "Which half a spliced item belongs to
is otherwise the item's own `ItemAuthor::Context` `phase`; the range form exists for the
reacquire edge rule." That is accurate, and it is the fact a slot design should lean on:
authorship already distinguishes compiler-spliced from consumer-written items, while
`acq_start..acq_end` (`eval/asm.rs:672-674`) is a raw INDEX range that does not. See Q2.

No outright comment/code disagreement found in `context.rs`, `z80_bus.rs`, or `lower_with`.

---

## Q2. Where a pre-grant slot would go, mechanically

### The three proofs need no change at all, and here is the code that decides it

A slot's statements would be spliced at an index strictly between `enter` and `exit`.
Therefore:

* `[context.escape]`: the walk is gated by `&|i| region.contains(i)` (`context.rs:363`) and
  the firing rules are gated by `region.in_body(idx)` (`context.rs:365-368`), whose upper
  bound is `body_end`. Both already cover any index between `enter` and `body_end`. A slot
  statement that branched out of the region would fire `Escape` with no edit. **No change.**
* `[context.entry-skip]`: the outside-transfer scan tests `region.contains(t) && t != entry`
  (`context.rs:441`) and the exported-label scan tests `region.contains(idx)`
  (`context.rs:449-450`). A label in the slot is `contains`, so a jump into it or an
  `export` on it fires. **No change.** Note `t != entry` is the whitelist for the corpus's
  spin-probe idiom: landing on the region's FIRST instruction is legal because it takes the
  whole acquire. A slot placed after the first instruction does not disturb that.
* `[context.reacquire]`: the nesting form reads marks only. **No change.** The branch form
  (`context.rs:374-378`) reads `in_acquire`, whose bound is `acquire_end`, so it is
  sensitive to WHERE the `AcquireEnd` mark is planted relative to the slot. This is a
  placement decision, not a rule change (see below).

Conclusion: the pairing proofs are not the cost of this feature. That corroborates aeon's
own 2026-09-10 correction banner on `LS-13b` (aeon `origin/master` `docs/DEFERRED_WORK.md:30476`),
independently re-derived here from the code rather than from the banner.

### What WOULD have to change

Four sites, all named exactly:

1. **`ast.rs:192-197`** (`ContextKind::Acquired`) must gain a seam. Today `acquire: Expr` is
   one value; a slot needs the compiler to know where inside the acquire the consumer's code
   goes. Any form that gives it that (a second field, a marker item, a `Code`-taking
   parameter) is a change here.
2. **`parser.rs:2150-2225`** (`context_decl`), specifically the field match at
   `parser.rs:2174-2188`, must admit whatever spells the seam. Today the unknown-field arm
   rejects every other bareword.
3. **`ast.rs:2269-2278`** (`AsmStmt::With`) and **`parser.rs:3115-3134`** (`asm_with`) must
   gain a place for the consumer's slot statements. Today `With` has exactly one body field.
4. **`eval/asm.rs:672-680`** is the lowering slot proper. Splice acquire-part-1, then the
   caller's statements via the EXISTING `lower_with_body` (`eval/asm.rs:788-800`, which just
   runs `lower_asm_stmt` per statement), then acquire-part-2, then plant `AcquireEnd`. That
   is a three-line change in the emission core.

### Two definition-site checks DO break, and nobody has named them

This is the part of Q2 that is not in any ledger row I found. Both of `lower_with`'s
definition-site checks read `buf.items[acq_start..acq_end]` as a raw index range:

```rust
// eval/asm.rs:747-764  [context.rte-acquire-pushes]
if rte && acq_ok {
    for item in &buf.items[acq_start..acq_end] {
        let CodeItem::Instr { ops, span: isp, .. } = item else { continue };
        if ops.iter().any(|o| matches!(o, CodeOperand::PreDec(Reg::A7))) { self.error(*isp, ...) }
    }
}
```

```rust
// eval/asm.rs:765-782  [proc.sr-undeclared]
if !rte && acq_ok && rel_ok
    && !crate::lower::sr_writes_round_trip(
        buf.items[acq_start..acq_end].iter().chain(&buf.items[rel_start..rel_end]),
    )
    && self.sr_reported_contexts.insert(ctx.to_string())
{ self.warn(decl_span, "[proc.sr-undeclared] context `{ctx}`'s acquire and release do not round-trip `sr` ...") }
```

If the consumer's slot statements land between `acq_start` and `acq_end`, both checks start
reading CONSUMER code and attributing it to the CONTEXT'S DECLARATION SPAN (`decl_span`,
`eval/asm.rs:654`). A push in the slot would be reported as the context's rte-acquire push;
an SR write in the slot would be charged against the context's round trip, at a span in a
different file. Both are error/warn tier and both would be wrong.

The fix is available and cheap, and the substrate already exists: filter by author rather
than by index. `splice_context_code` re-authors its items `ItemAuthor::Context { name, phase }`
(`eval/asm.rs:829-832`, `value.rs:391-399`), and the consumer's slot statements would keep
`ItemAuthor::User`. So both checks become "the `Context`-authored items in this range"
instead of "this range."

**This is a finding, not a fix: nothing here was changed.** It is the concrete answer to
"say whether it is more invasive than a slot in the lowering." It is not much more, but it
is not zero, and the two sites are not obvious from the outside.

### Where `AcquireEnd` should fall, and the one thing that depends on it

Two placements are available and they are not equivalent:

* **`AcquireEnd` after acquire-part-2** (slot inside the acquire range). Then
  `in_acquire` covers the slot, so a back-edge from the body into the slot fires
  `[context.reacquire]` (`context.rs:374-378`), which is the right answer: re-entering the
  slot re-runs the consumer's pre-grant work without a matching release. And
  `region_acquires_bus` (`z80_bus.rs:196-202`) still sees the bus request, because it reads
  `items[region.enter..region.acquire_end]`.
* **`AcquireEnd` after acquire-part-1** (slot and spin both in the body range). Then the
  spin's own `bne` back-edge is a body-to-body edge (fine), the slot gets no re-entry
  protection, and `region_acquires_bus` still sees the request (it is in part 1). Strictly
  weaker for no gain I can measure.

The first placement is the one the existing rules already reward.

### One more thing `region_acquires_bus` says about the register demand

`LS-13b` requires the form to emit through a caller-supplied register. That does not break
the declared tier, and the reason is worth recording because it is load-bearing and fragile:

```rust
// z80_bus.rs:196-202
pub fn region_acquires_bus(items: &[CodeItem], region: &crate::context::Region) -> bool {
    items[region.enter..region.acquire_end].iter().any(|it| {
        matches!(it, CodeItem::Instr { mnemonic, ops, .. }
            if bus_toggle(mnemonic.as_str(), ops) == Some(STOPPED))
    })
}
```

`bus_toggle` (`z80_bus.rs:150-165`) requires a `move` with an `Imm(0x0100)` source and a
destination naming `Z80_BUS_REQUEST` or `$A11100` (`is_z80_bus_request`, `z80_bus.rs:130-137`);
a register-indirect destination returns `false`. So boot's `move.w d7,(a1)` region would NOT
be recognised as bus-acquiring.

It does not matter TODAY because `bus_contexts` is a tree-wide UNION over every region
(`corpus_contracts.rs:1160-1171`), and the other 22 sites spell the immediate form. The seed
that feeds `BusEntry::Held` is `n.requires.iter().any(|c| bus_contexts.contains(c))`
(`corpus_contracts.rs:1252-1255`), so `z80_stopped` stays identified as a bus context.

**Finding Q2-a, booked not built:** that identification is a tree-wide union over spellings,
so it is silently load-bearing on at least one immediate-spelled site existing. If a future
form let every site emit through a register, `bus_contexts` would go empty and every
`requires(z80_stopped)` proc would silently fall back to the `Unknown` seed, disabling
`[bus.released-at-return]` for all of them with no diagnostic. There is no check that a
declared bus context is ever recognised. Per the brief I have added no gate; this is the
finding.

---

## Q3. What aeon's boot actually asks for

Read at aeon `origin/master` `3794c99b`, `engine/system/boot.emp`, proc `EntryPoint`
(declared at `:38`). The file is 369 lines at that revision. Line numbers are that revision's.

The hold runs from the bus request at `:130` to the bus release at `:170`. Every statement
in and around it, classified:

| line | statement | class |
|---|---|---|
| 129 | `move.w  d0, (a2)` (assert Z80 reset) | **(a) before the acquire** |
| 130 | `move.w  d7, (a1)` (request Z80 bus) | **the acquire** (bus request) |
| 131 | `move.w  d7, (a2)` (release Z80 reset) | **(b) PRE-GRANT SLOT** |
| 133 | `.wait_z80:` | (c) grant-spin (label) |
| 134 | `btst    d0, (a1)` | (c) grant-spin |
| 135 | `bne     .wait_z80` | (c) grant-spin (backward branch) |
| 141-145 | comptime `if SOUND_DRIVER_ENABLED == 1 { move.w #Z80_SOUND_SIZE-1, d1 } else { moveq #Z80_IDLE_SIZE-1, d1 }` | (d) body; emits exactly one instruction |
| 146 | `.load_z80:` | (d) body (label) |
| 147 | `move.b  (a5)+, (a0)+` | (d) body |
| 148 | `dbf     d1, .load_z80` | (d) body (backward branch) |
| 165 | `move.w  d0, (a2)` (assert reset) | (d) body |
| 166 | `moveq   #25, d2` | (d) body |
| 167 | `.ym_delay:` | (d) body (label) |
| 168 | `dbf     d2, .ym_delay` | (d) body (backward branch) |
| 169 | `move.w  d7, (a2)` (release reset) | (d) body |
| 170 | `move.w  d0, (a1)` (release bus) | **(e) the release** |

### The counts

* **Statements that must sit in the pre-grant slot: exactly 1.** Line 131,
  `move.w d7, (a2)`. It is not a loop, not a label, and not a branch target. Verified by
  reading every line between `:130` and `:133` at the revision; there are no others.
* Instructions in the ordinary body between the grant and the release: **7**
  (one of `:142`/`:144`, then `:147`, `:148`, `:165`, `:166`, `:168`, `:169`), plus **2**
  local labels (`.load_z80`, `.ym_delay`).
* Backward branches strictly between the request (`:130`) and the release (`:170`): **3**.
* Backward branches in the ORDINARY BODY: **2** (`:148`, `:168`).

### Where my measurement differs from the framing I was handed

The brief asked me to test, not assume, a prior round's conclusion: "boot needs only ONE
statement in that slot, and both of boot's backward branches live in the ordinary body."

* **The count of one is CONFIRMED.** One statement, `:131`, straight-line, unlabelled.
* **"Both of boot's backward branches" is true only under a reading that should be stated.**
  There are THREE backward branches inside the hold, not two. The third is `bne .wait_z80`
  at `:135`, and it is the grant-spin's own. The claim is right about the branches boot
  would still be WRITING by hand under a bracket, because the spin's branch becomes
  compiler-emitted. But a form designed against the sentence "there are two" would be
  designed against the wrong region shape: the spin's back-edge is precisely what
  `Region::in_acquire`'s source-side exclusion at `context.rs:374-378` exists for, and a
  form that moved the spin out of the acquire range would change which rule covers it.
  Stated as a count: 3 backward branches in the hold, 2 of them consumer-written, 1 of them
  the construct's own.
* aeon's `LS-13b` row gives the near interleave's span as `:129-133`
  (`docs/DEFERRED_WORK.md:30476`). `:129` is the reset ASSERT and it precedes the bus request
  at `:130`, so it is before the acquire, not in the slot. The row's own paragraph (2) names
  the right single statement (`move.w d7,(a2)`); only the line range is one line wide.

### The release side needs nothing

`:169` (`move.w d7, (a2)`, release reset) sits between the last body work and the bus release
at `:170`. It is an ordinary trailing body statement: a bracket would emit its release after
it with no new form. **There is no post-body slot requirement.** Measured by reading
`:165-170`; the only statement between the last `dbf` and the bus release is `:169`.

### The byte price of adoption, hand-derived

Not built (this parcel does not build). Derived from the 68000 encoding and from
`boot.emp:15-19`, which states that the `$A1xxxx` hardware ports force `abs.l` regardless of
comptime knownness.

* The declared context's spelling (aeon `engine/z80_bus.emp:144-153`):
  `move.w #$0100, Z80_BUS_REQUEST` = 8 bytes; `btst #0, Z80_BUS_REQUEST` = 8; `bne` = 2;
  `move.w #$0000, Z80_BUS_REQUEST` = 8. **Total 26 bytes.**
* Boot's hand spelling: `move.w d7,(a1)` = 2; `btst d0,(a1)` = 2; `bne` = 2;
  `move.w d0,(a1)` = 2. **Total 8 bytes.**

So an immediate-spelled bracket costs boot **+18 bytes** in a byte-verified ROM path. That
is the measured content of `LS-13b`'s surviving demand that the form emit through a
caller-supplied register, and it is a verification-and-quality cost, not a safety one.
UNVERIFIED by a build, by instruction; flagged for the controller if a byte figure is to be
cited anywhere binding.

---

## Q4. Is boot the only site, or is there a population?

Enumerated at aeon `origin/master` `3794c99b` over the 1803 tracked files at that revision
(`git ls-tree -r --name-only | wc -l`), using `git grep` at the ref. No filesystem walk was
used: this repo hosts worktrees inside itself and a disk walk would match many checkouts.

### Bracketed sites: 22

`git grep -n 'with z80_stopped' origin/master -- '*.emp'` returns 28 hits. Six are prose
(comment lines). The 22 code sites, by file:

| file | code sites |
|---|---|
| `engine/sound/sound_api.emp` | 6 (`:111`, `:145`, `:265`, `:307`, `:417`, `:587`) |
| `engine/system/vblank.emp` | 6 (`:129`, `:137`, `:292`, `:347`, `:355`, `:463`) |
| `engine/level/section.emp` | 3 (`:309`, `:327`, `:569`) |
| `engine/level/bg.emp` | 2 (`:162`, `:223`) |
| `engine/system/boot.emp` | 1 (`:261`, the YM key-off block) |
| `engine/system/controllers.emp` | 1 (`:39`) |
| `engine/level/parallax.emp` | 1 (`:1863`) |
| `engine/debug/sound_debug.emp` | 1 (`:115`) |
| `games/sonic4/test/ojz_scroll_test.emp` | 1 (`:814`) |

Four of the 22 carry a comptime gate (`with z80_stopped if SOUND_DRIVER_ENABLED == 0` at
`section.emp:327`, `vblank.emp:137`, `vblank.emp:355`; plus `controllers.emp:39` which is
explicitly annotated unconditional). Total bracket population tree-wide, all contexts:
22 `z80_stopped`, 7 `ints_off`, 1 `ints_off_until_rte`.

### Hand-rolled sites: exactly 1

Instrument: `git grep -nE 'Z80_BUS_REQUEST|Z80_RESET|\$?A11100|\$?A11200' origin/master`
over ALL tracked files, then a case-insensitive `a112?00` sweep restricted to `*.emp` and
`*.asm`, then a `stop_z80|start_z80|stopZ80|startZ80` sweep. In CODE files
(203 `.emp`, 3 `.asm`) the complete hit set is:

| path:line | what it is |
|---|---|
| `engine/system/constants.emp:574-575` | the two constant definitions |
| `engine/z80_bus.emp:145,147,152` | the context declaration's acquire/release |
| `engine/system/boot_data.emp:37,155,156` | the `use`, and `dc.l Z80_BUS_REQUEST` / `dc.l Z80_RESET` in the `BootData` table |
| `engine/system/boot.emp:94` | a COMMENT (`a1=Z80_BUS_REQUEST` in the movem-preload note) |

Everything else is `docs/`, `tools/` and `wiki/` prose. The three `.asm` files
(`engine/debug/debugger.asm`, `games/demo/game_root.asm`, `games/sonic4/game_root.asm`)
contain no hit at all. `stop_z80`/`start_z80` survive only in comments.

The symbol grep alone does NOT find boot's hold: boot's request and release are
`move.w d7,(a1)` and `move.w d0,(a1)` and neither instruction names the register. The
durable route is the data table. `git grep -n BootData origin/master -- '*.emp' '*.asm'`
gives one consumer of the preload head: `boot.emp:96` `lea BootData(pc), a5` followed by
`movem.l (a5)+, a0-a4` at `:98`. (`boot.emp:215` re-anchors on `BootData_PostBlob`, a
different label past the blob; `vdp_init.emp:30` reads `BootData_VDPRegs`, also different.)
So `$A11100`/`$A11200` reach a register in exactly one place in the tree.

**Population of hand-rolled 68k Z80-bus holds at this revision: 1.** `EntryPoint` in
`engine/system/boot.emp`. This re-derives aeon's own census figure of 23 holds
(22 bracketed + 1 hand-spelled, `engine/z80_bus.emp:31`) by an independent route.

Does that one site interleave anything, and does a one-statement slot cover it? Yes and yes,
per Q3: one statement, straight-line, unlabelled. **There is no second hand-rolled site
whose needs could widen the requirement.** The requirement is derived from a population of
one, which is itself the most important fact about how wide the form should be.

What I did NOT measure, stated as such: I enumerated sites that MANIPULATE the bus. I did not
enumerate sites that merely CONSUME a hold (reads and writes of `Z80_RAM` under someone
else's bracket). Those cannot need a pre-grant slot, so they are out of scope, but "measured
zero hand-rolled consumers" is not a claim I am making.

---

## Q5. Where `z80_stopped` is specified, and what a change would have to update

### The brief's directive could not be carried out as written

The brief says: search empyrean, "then inspect the `docs/SIGIL_*.md` hits."
`git -C empyrean grep -n z80_stopped origin/main` returns hits, but **none of them are in
`docs/SIGIL_*.md`.** Every hit is in `wiki/` (rendered pages and sweep workpapers) or in
`wiki/specs/*.md` factsheets. I checked the two candidate language specs directly:
`docs/SIGIL_SPEC2_LANGUAGE.md` has 21 lines containing the substring "context" and not one
of them is about the `context` item or the `with` bracket (they are "contextual opener",
"contextual bareword", "data context"). `docs/SIGIL_CORE_SPEC.md` has 2, both unrelated.

**Finding Q5-a: the `context` / `with` construct is not specified in empyrean at all.** Its
specification lives in sigil, at `docs/superpowers/specs/2026-08-03-contract-unification-spec.md`
§3.1-3.2 (`:68-112`) with the delta at `docs/superpowers/specs/2026-08-04-contract-delta-spec.md`
§2 (`:44-60`). Notably, empyrean's own "the whole language on one page" inventory
(`docs/SIGIL_SPEC2_LANGUAGE.md:938-947`) lists every declaration form the language has and
does not list `context`; §10's declaration line at `:940` ends at `comptime test` and doc
comments. So the construct is already undocumented at the suite level, and a new form would
compound that rather than create it.

### The binding constraint a new form must satisfy

`docs/SIGIL_SPEC2_LANGUAGE.md:951` at empyrean `origin/main`, the headroom rule:

> the statement-leading reserved set above is **closed** ... All future constructs must
> instead enter the language as **contextual item-position openers** ... as attributes
> (`@name`), or as prelude-resolved builtins.

`with` itself already obeys this (`parser.rs:2754` fires only on `with` + identifier). Any
slot spelling must too: no new statement-leading reserved word, and any new context-body
bareword must follow the `acquire`/`release`/`granted`/`released_by_rte` pattern of being a
keyword only inside a `context` body.

### Every artifact a new form would have to update

**sigil, at `26de6eb7`:**

| path | why |
|---|---|
| `crates/sigil-frontend-emp/src/ast.rs:192-197` | `ContextKind::Acquired` gains the seam |
| `crates/sigil-frontend-emp/src/ast.rs:2269-2278` | `AsmStmt::With` gains the slot |
| `crates/sigil-frontend-emp/src/parser.rs:2150-2225` | `context_decl` field match |
| `crates/sigil-frontend-emp/src/parser.rs:3115-3134` | `asm_with` |
| `crates/sigil-frontend-emp/src/eval/asm.rs:666-701` | the emission core |
| `crates/sigil-frontend-emp/src/eval/asm.rs:747-782` | the two definition-site checks that read `acq_start..acq_end` by index (Q2) |
| `crates/sigil-frontend-emp/src/context.rs` | doc only, if `AcquireEnd` placement changes what `in_acquire` means; the RULES need no edit |
| `crates/sigil-frontend-emp/src/z80_bus.rs:180-202` | `region_acquires_bus`'s doc comment reasons explicitly about the `Enter..AcquireEnd` range and why it is not `Enter..BodyEnd`; a slot inside that range changes what that paragraph is describing |
| `crates/sigil-frontend-emp/tests/context_brackets.rs` | the construct's own test file (~30 `z80_stopped` occurrences) |
| `crates/sigil-frontend-emp/tests/lower_proc.rs:1410-1430` | the nested-region acquire/release range test |
| `crates/sigil-cli/tests/contract_closure_corpus.rs:1270-1330` | the corpus bracket census, which pins per-proc region lists |
| `docs/superpowers/specs/2026-08-03-contract-unification-spec.md:68-112` | §3.1 declaration grammar and §3.2 introduction forms |
| `docs/superpowers/specs/2026-08-04-contract-delta-spec.md:44-60` | §2, B-prime-1 |
| `docs/OVERSEER-REFERENCE.md:2343-2348` | states the exception as "a shape `with z80_stopped` cannot take" |

**empyrean, at `cabaa0d8`:**

| path | why |
|---|---|
| `docs/SIGIL_SPEC2_LANGUAGE.md:938-947` | §10 inventory; `context`/`with` are missing today and a new form is the moment to add them |
| `wiki/specs/2026-08-06-emp-language-factsheet.md:179-191` | states the bracket's surface and names all four `[context.*]` ids |
| `wiki/specs/2026-08-31-z80-on-genesis-factsheet.md:99,114` | quotes the acquire/release code block verbatim, and cites a bracket-site count |
| `wiki/emp/contracts.html:847-972,1401` | the rendered teaching page, including a live `[context.escape]` diagnostic string |
| `wiki/genesis/two-cpus.html:565,671` | quotes `pub context z80_stopped` verbatim |
| `wiki/aeon/section-0-boot.html:930-934,1431` | describes boot's YM key-off bracket |
| `wiki/aeon/section-02-vdp-frame.html:672-676,1382-1391` | quotes the gated bracket form |

**aeon, at `3794c99b`** (consequential, not a prerequisite):

| path | why |
|---|---|
| `engine/z80_bus.emp:1-154` | the header is the census and it names boot as THE exception, in six places |
| `engine/system/boot.emp:150-171` | the `.ym_delay` comment already anticipates the bracket and says its own linkage becomes load-bearing then |
| `docs/DEFERRED_WORK.md:30476` | `LS-13b` |
| `docs/lane-status.json` | the `LS-13b` row, `state: blocked` |
| `tools/test_z80_bus_hold_mask_census.py`, `tools/test_sound_bus_hold_mask_lint.py` | both classify boot's hold as the hand-spelled one |

### One cross-repo fact that does not hold

aeon's `LS-13b` row says the question was "Routed to sigil the same day, where it is booked
as `EMP-Z80-HOLD-INTERLEAVE-FORM`," and `docs/lane-status.json`'s `LS-13b` row reads
`blockedBy: "sigil, row EMP-Z80-HOLD-INTERLEAVE-FORM"`.

**That identifier appears nowhere in sigil at `origin/master` `26de6eb7`.**
`git grep -l 'EMP-Z80-HOLD' origin/master` exits 1 with no output. aeon's board therefore
names a sigil row that does not exist at sigil's published tip. Whether the row exists in an
unpushed tree I cannot see; what is measurable is that it is not at the revision aeon's
blocker points at. Reported, not repaired: this note changes nothing in aeon.

---

## What a form must admit

Stated as a measured requirement, from a population of one hand-rolled site at aeon
`origin/master` `3794c99b`:

1. **Exactly one straight-line statement between the bus-request write and the grant-spin.**
   `boot.emp:131`, `move.w d7, (a2)`. Not a loop, not a label, not a branch target.
2. **An acquire and a release that can emit through registers the caller supplies.** Boot's
   four instructions are 8 bytes; the immediate-spelled context's are 26. The gap is 18
   bytes in a byte-verified ROM path (hand-derived encoding, not built).
3. **The compiler still owns the release on every exit path.** This is what the three proofs
   rest on, and nothing measured here puts it at risk: all three range over
   `(enter, exit)` or `(enter, body_end)` and already cover any slot placed inside.

## What is NOT required, and the measurement that rules it out

* **A slot that admits a loop, a label, or a branch target.** Ruled out by the line-by-line
  classification of `boot.emp:129-170`: the only pre-grant statement is `:131`, and it is a
  single `move`. The two `dbf` loops (`:148`, `:168`) and their labels are AFTER the grant
  spin, in the region the bracket's ordinary body already admits and already proves. A
  slot grammar of "one or more asm statements" is enough and a slot grammar of "one
  statement" is enough for this site. Nothing measured here demands loop-shaped slot
  contents. (aeon's own `LS-13b` correction banner asserts a wider requirement, "it must
  admit a labelled backward branch"; that banner is reasoning about the FAR interleave at
  `:165-169`, which this measurement places in the ordinary body, not in any slot. If the
  designed form takes that wider requirement it should take it as taste, not as a measured
  need.)
* **A post-body / pre-release slot.** Ruled out by `boot.emp:165-170`: everything before the
  bus release is ordinary body, and the bracket emits its release after the body already.
* **Changes to `[context.escape]`, `[context.entry-skip]`, or the nesting form of
  `[context.reacquire]`.** Ruled out by the index predicates at `context.rs:209-231` and the
  gates at `context.rs:363-368,441,449-450`, all of which are bounded by `exit` / `body_end`
  and therefore already cover a slot.
* **A relaxation of any soundness argument.** The `[bus.*]` net's register-indirect bailout
  (`z80_bus.rs:130-146`) is a separate inferred tier and is not touched by a bracket-shape
  change; the bracket proofs never recognise an operand at all.
* **A wider surface justified by "there might be other sites."** There are no other sites.
  Population measured at 1, by two independent instruments (symbol/address grep over 1803
  tracked files; the `BootData` consumer route). An over-wide surface is close to
  unremovable once the game's source is written in it.

## Open questions and what could not be measured

1. **`AcquireEnd` placement is a real choice with a real consequence and it is unresolved.**
   Placing the slot inside `(enter, acquire_end)` buys re-entry protection via
   `context.rs:374-378` and keeps `region_acquires_bus` working; placing it outside buys
   nothing I can measure. Not a measurement gap; a design call that this note declines to
   make.
2. **The two definition-site checks at `eval/asm.rs:747-782` read the acquire by index.**
   Finding Q2-b above. No gate added (instructed), no fix applied.
3. **No check exists that a declared bus context is ever RECOGNISED as one.** Finding Q2-a.
   `bus_contexts` is a tree-wide union; if every site emitted through a register it would go
   empty and disable `[bus.released-at-return]` tree-wide with no diagnostic. Booked here.
4. **The +18 byte figure is hand-derived from the 68000 encoding, not built.** This parcel
   does not run `cargo` or `build.sh`. TAGGED for the controller if the number is to be
   cited anywhere binding: the confirming command is a byte-diff of the ROM after an
   experimental adoption, which is a build parcel, not this one.
5. **Runtime behaviour was not confirmed and no emulator was touched**, by standing
   invariant. Nothing in this note rests on a run.
6. **`EMP-Z80-HOLD-INTERLEAVE-FORM` does not exist at sigil `origin/master` `26de6eb7`.**
   Whether it exists somewhere unpushed is not measurable from here. aeon's `lane-status.json`
   `blockedBy` string points at it.
7. **Sites that CONSUME a hold without manipulating the bus were not enumerated.** Out of
   scope for a pre-grant slot; stated so that the population-of-one claim is not read wider
   than it was measured.

---

## Overseer correction, 2026-09-16: requirement 2 is REFUTED, and the refutation is the design's load-bearing fact

Added at the sigil overseer seat after recomputing this note's claims with this lane's own
instruments. Everything above stands as measured except **"What a form must admit" item 2**,
*"an acquire and a release that can emit through registers the caller supplies"*. That is not a
requirement of the form. It is the constraint this lane's `docs/OVERSEER-REFERENCE.md` already
records as having outlived two reasons, and the measurement below is a third reason that points
the OTHER WAY: a register-emitting form is the one shape the design must NOT take.

**The code that decides it**, `crates/sigil-frontend-emp/src/z80_bus.rs:153-166`, read at sigil
`26de6eb7`:

```rust
fn bus_toggle(mnem: &str, ops: &[CodeOperand]) -> Option<BusState> {
    if mnem != "move" { return None; }
    let dst = ops.last()?;
    if !is_z80_bus_request(dst) { return None; }
    match ops.first() {
        Some(CodeOperand::Imm(0x0100)) => Some(STOPPED),
        ...
```

Recognition needs **both** a destination that resolves to the bus-request address and a source
that is the literal immediate `$0100`. Boot's `move.w d7, (a1)` supplies neither: the destination
is register-indirect (the documented soundness bailout) and the source is a register. So
`bus_toggle` returns `None`, and `region_acquires_bus` (`z80_bus.rs:196-202`), which scans the
emitted items of `enter..acquire_end` for exactly that toggle, returns false.

**The consequence is a silent loss of a whole tier, not a missed lint.**
`corpus_contracts.rs:1160-1169` builds `bus_contexts` as a tree-wide union of the contexts
`region_acquires_bus` recognises, and `:1253` seeds a proc `BusEntry::Held` only when its
`requires(...)` names one of them. A `z80_stopped` whose acquire emitted through registers would
never enter that set, so **every `requires(z80_stopped)` proc in the tree would silently seed
`Unknown` instead of `Held`**, and the `[bus.*]` crash-class analysis would quietly stop saying
anything. Nothing diagnoses this: an empty `bus_contexts` is indistinguishable from a tree with
no bus contexts in it. That is finding Q2-a, and this is the mechanism that makes it matter.

**So the third reason for the absolute spelling, and the first one that holds.** Reason one
(aeon's register economy for the reset path) was refuted by aeon. Reason two (a DMA-window
cycle-count hazard) was refuted by this lane on ordering and direction. Reason three is that
**the absolute spelling is what our own bus analysis can see**, and a register spelling blinds it
without a word of complaint. That is a constraint about this toolchain rather than about the
hardware, which is why neither hardware argument could ever have established it.

**What this does to the byte figure.** The hand-derived 18 bytes at boot (item 4 of the open
questions, still unbuilt and still not to be cited bindingly) stops being a cost with nothing
bought. It is the price of a machine-checked bus-state property at the one site that has never
had one. That is a trade a person can weigh; "18 bytes for a tidier spelling" is not.

**And it keeps the general shape this note already names**, aeon's own lesson pointed at a third
instance: a constraint whose reason is wrong is more fragile than one with no reason at all,
because refuting the reason looks like refuting the constraint. This one has now survived two
wrong reasons. It is right, and for neither of them.
