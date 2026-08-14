# sigil — lens-panel adjudication packet

**Review SHA:** `476a5bd9` (pinned; worktree `.worktrees/sigil-lens`, branch `review/sigil-lens-sweep`)
**Corpus:** 15 crates, ~188,000 lines of Rust — the from-scratch assembler/linker that IS the build
for the Aeon Genesis engine since the Spec-5 Stage-2 flip.
**Panel:** 23 seats, adapted for a Rust compiler. Three performance seats were seated deliberately
(an assembler's speed is developer experience, and sigil had never been profiled).
**Adjudication:** every finding below was re-verified by the overseer against the pinned tree.
This packet is written for an implementing agent: each item states what to change and where.

> **COVERAGE NOTE — read this first.** This packet covers the **12 seats that completed** before the
> review session ran out of context: **P1b, CGa, CGb, RELAX, Va, Vb, IR, ARCH, LINK, HALF, B1, ERR**.
> Eleven further seats were dispatched and were still running: P1a, P2, GATE, TEST, COMPTIME, SAFE,
> A2, B2, FUZZ, ASFE, CACHE. Their results are **not** in here. Most notably absent: the
> **golden/pin machinery audit (GATE)** and the **test-suite vacuity counts (TEST)** — the two seats
> that would tell you how much the existing safety net is worth. Treat this as a strong partial.
>
> Findings S1-S8 were overseer-verified against the pinned tree. Findings S9-S13 (added from the
> later seats) are reported as the seats found them, with their own evidence cited; they were not
> independently re-verified.

---

## 1. Headline

**Two findings dominate. One is a live correctness class; the other is a 3x build speedup.**

sigil is, structurally, better engineered than the sweep expected — a clean layered crate DAG with a
machine-enforced guard test, an IR that is genuinely target-neutral, a relaxation fixpoint whose
termination is *proven* rather than hoped, a hand-verifiable Z80 T-state table, and zero feature
flags (build shapes are runtime profiles, so every shape is testable without a rebuild). Those are
real and are recorded in §5.

The defects that matter are concentrated:

1. **The ADDX miscompilation class was never fixed — only one site was.** Four more exact-alias
   encodings exist, all reachable from ordinary `.emp` source, all silent.
2. **Two-thirds of build wall-clock is thread-lifecycle overhead**, not computation.
3. **`clobbers(...)` on Z80 is syntax-checked only**, while the sound driver's headers claim it is
   machine-checked and cite a prior bug as the reason to trust it.

---

## 2. Confirmed defects, ranked

### S1 — Four exact-alias miscompilation classes; the ADDX fix patched a site, not the class · CRITICAL
**Seat:** CGa · **Overseer-verified: YES** (R4 confirmed directly; all four Capstone-decoded by the seat)

`encode_ea` (`crates/sigil-isa/src/m68k.rs:1265`) resolves an addressing mode to `(mode, reg, ext)`
with **no model of which EA classes each instruction accepts**. `Field` distinguishes only
Source/Dest and rejects only `#imm` and PC-relative in Dest. Address-register mode `001` is accepted
everywhere. Four instructions have opcode neighbours reachable by exactly that mode:

| source | sigil emits | the 68000 executes | consequence |
|---|---|---|---|
| `bset d0,a1` | `01C9` | `movep.l d0,$8(a1)` | **memory write** + 2 bytes emitted vs 4 consumed |
| `sne a3` | `56CB` | `dbne d3,…` | backward branch to garbage + desync |
| `eor.w d0,a1` | `B149` | `cmpm.w (a1)+,(a0)+` | two address registers advance, two stray reads |
| `pea d0` | `4840` | `swap d0` | no push; stack depth wrong for the rest of the routine |

The MOVEP case is **worse than the original ADDX bug**: sigil emits 2 bytes where MOVEP consumes 4,
so the displacement it eats is the next instruction's opcode word — total instruction-stream desync
from that point on.

`pea`/`swap` is self-documented: `m68k.rs:825` says "*shares its base word with `pea`; dispatched by
mnemonic*", `:891` is `Mnemonic::Pea => 0x4840`, `:929` is `Ok((0x4840 | dn)…)`. The collision was
known and left.

**The `eor` case is the process finding.** The regression test written as the ADDX post-mortem —
`alu_ea_rejects_address_register_destination` (`m68k.rs:1363`) — covers `add`/`sub`/`and`/`or` and
**omits `eor`**, which is handled by its own untouched arm at `m68k.rs:341-351`.

**Why nothing caught it:** the golden corpus (`crates/sigil-isa/tests/corpus_m68k/`) is minted by
feeding snippets to real `asl`. **An illegal form can never appear in it** — `asl` refuses to
assemble one. The oracle is positive-only by construction: ~200 positive byte assertions to 12
negative, and 9 of the 12 are the single ADDX post-mortem test.

**Fix (the durable one, from the seat):** an EA-class function in `sigil-isa` —
`fn ea_class(op: &Operand) -> EaClass` plus a per-mnemonic `allowed: EaClassSet` consulted inside
`encode_ea`. Adding a `Mnemonic` variant then fails to compile until its EA classes are declared,
exactly as `writes_last_operand` (`m68k.rs:92`) already forces classification. Everything in S1 and
S2 is one `data-alterable` / `control` / `alterable` predicate away.

### S2 — Silent acceptance of ~25 further illegal operand forms · HIGH
**Seat:** CGa

Same root cause, no exact alias — these decode as illegal/undefined, so they trap at runtime rather
than corrupting silently. Still `encode` returning `Ok` where the ISA has no encoding:
`tst.w a0`, `clr.w a0`, `addq.b #1,a0`, `addi.w #1,a0`, `and.w a0,d0`, `cmp.b a0,d0`, `muls.w a0,d0`,
`lea (a0)+,a1`, `lea #$1000,a0`, `jmp (a0)+`, `pea -(a0)`, `asl.w d0`, `movem.l d0-d7,(a0)+`.

Plus two specific defects worth separate mention:
- **`encode_alu_ea` validates the destination as if it were a source** (`m68k.rs:406` passes
  `Field::Source` for both directions), so `add.w d0,#5` and `add.w d0,(Lbl,pc)` encode. This also
  **falsifies a load-bearing comment**: `sigil-frontend-as/src/eval.rs:3440` asserts "(d16,PC) is
  illegal as a DESTINATION EA (`encode_ea` rejects it there)" and routes operands on that basis.
- **`movem` passes `Field::Dest` in both directions**, which *falsely rejects* the legal
  `movem.l (Lbl,pc),d0-d7` (MOVEM load explicitly permits PC-relative).

### S3 — 39,541 thread spawns per build; two-thirds of wall clock is not computation · HIGH (performance)
**Seat:** P1b · **Overseer-verified: YES**

```rust
// crates/sigil-frontend-emp/src/eval/mod.rs:1815
pub(crate) fn run_on_eval_stack<T, F>(f: F) -> T {
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(EVAL_STACK_BYTES)   // 64 MiB, mod.rs:1807
            .spawn_scoped(scope, f) … handle.join()
    })
}
```

**30 call sites** — `layout.rs` 16, `eval/mod.rs` 7, `lower/*` 6, `guards.rs` 1 — invoked per `align`
directive, per data-item length, per array bound, per proc instantiation, per struct fold. Each
spawns a fresh 64 MiB-stack OS thread, does one small piece of comptime work, joins, tears down.
Nothing runs concurrently; the caller blocks on `join()` every time.

Measured on the real engine (`sigil build --aeon … --native --game sonic4`):

| | sonic4 | demo |
|---|---|---|
| wall | 12.5-13.7 s | 8.1-8.6 s |
| user + sys | ~4.3 s | ~2.5 s |
| **CPU** | **32-34%** | **30-31%** |
| spawns | **39,541** | 23,273 |

Per-spawn cost cross-checked two ways — a standalone microbenchmark (~385 µs) and the marginal cost
between the two real builds ((12.7−8.1)/(39541−23273) ≈ 283 µs). 39,541 × ~300 µs ≈ 12 s against a
4.3 s CPU floor.

**The 64 MiB stack is not a safety mechanism.** `MAX_CALL_DEPTH = 512` (`eval/mod.rs:45`) is the
runaway-recursion guard; the stack is headroom only. Removing the per-call spawn weakens nothing.

**Fix:** one long-lived big-stack worker fed over a channel, or the `stacker` crate's `maybe_grow`
(extends the current thread's stack only on deep recursion — lower risk, no lifetime surgery).
**Secondary, same call sites:** each call also constructs a fresh `Evaluator` via
`Evaluator::with_file` (`eval/mod.rs:756`) whose `index_items()` rebuilds six `HashMap` indexes from
scratch — hoist to one per file.

**Expected: ~12.7 s → near 4.3 s. Roughly 2.5-3x on every build.** Do this first; it changes the
answer to several other questions (see §4).

### S4 — Z80 `clobbers(...)` is syntax-validated only, while the sound driver claims it is verified · HIGH
**Seat:** Vb · **Overseer-verified: YES** (both sides read directly; seat built and ran a repro)

```
sigil  lower/proc.rs:1164   "The undeclared-write LINT below is 68k-only … so on Z80
                             reglist validation is the whole job."
                    :1186    return;          ← before any write-detection

aeon   sound_psg.emp:79     "The full register contract is machine-checked: … clobbers(…)
       sound_fm.emp:76       the destroyed scratch … (a false clobber comment caused a
                             prior bug — the checker closes that class)."
```

A Z80 proc writing `iy` with no declaration anywhere compiles clean, **zero diagnostics**. The
identical 68k shape fires `[proc.clobber-undeclared]` immediately.

What makes it insidious is that the claim is two-thirds true: `preserves(...)` **is** verified on Z80
(`z80_preserves.rs`, wired at `lower/proc.rs:904`) and `out(...)` **is** verified
(`z80_out_verify.rs`, wired at `corpus_contracts.rs:782`). Only `clobbers` is not — and the header
groups all three. The claim governs the register contracts across ~9,600 lines of Z80 sound driver,
and cites the exact bug class the missing checker would catch.

`crates/sigil-frontend-emp/tests/z80_contracts.rs` has tests for `[proc.clobber-invalid]` (bad
register *name*) and **zero** for an undeclared-write diagnostic, because none exists.

**Related, same seat:** `[context.unsatisfied]` (`requires(vblank)`) only inspects call sites whose
target resolves to a **direct symbolic** name (`corpus_contracts.rs:~1096` — `let Some(target) =
call_target_sym(ops) else { continue };`). Any `requires(vblank)` callee reached through an indirect
`jsr (aN)` is never checked. Code-confirmed; no live corpus exploitation found.

**Also recorded:** 68k clobber closure only ever flags *under*-declaration (`closure.rs:418`). An
over-claimed `clobbers(d0-d7)` fires nothing, on either CPU. That is the safe direction and no aeon
comment claims otherwise — but it is the general answer to "does sigil prove equality or a superset?"
It proves a superset.

### S5 — Eight Z80 instructions exist in the analyzers with no encoder arm · MEDIUM
**Seat:** CGb

`rst` was the known instance (blocked ~116 B of sound-driver reclaim). It has **seven siblings**:
`rlca`, `rla`, `rra`, `daa`, `cpl`, `ccf`, `halt`. All appear in `z80_preserves.rs:165`,
`flag_check.rs:188` and friends; none has a `Mnemonic` variant, a `z80_mnemonic()` entry, or an
`encode()` arm. Writing one is a loud parse error, never wrong bytes. Zero uses in the live driver
today — but every one is a 1-byte primitive a size-reclaim pass reaches for (`RLA` is 4T/1 byte
against the CB-prefixed `RL A` at 8T/2 bytes).

`rst` implementation is small and scoped by the seat: `RST p` = `0xC7 | (p & 0x38)`, ~20-40 lines
across `z80.rs`, `lower/code.rs`, `z80_cycles.rs` (11 T-states), and a corpus regeneration.

Also unimplemented, same failure mode (loud): DD/FD-prefixed CB shift/rotate (`rlc (ix+d)` etc. — the
`bit`/`res`/`set` indexed forms *are* implemented, so this is asymmetric), and four of eight ALU ops
lack indexed forms (`adc`/`sbc`/`and`/`xor` with `(ix+d)`).

### S6 — Relaxation is sound; its boundary tests are not · MEDIUM
**Seat:** RELAX

Good news first, and it is substantial: **termination is proven** (rung indices grow-only, candidate
lengths checked non-decreasing in *release*, placement idempotent — the pass cap at `relax.rs:965` is
unreachable dead code). **Determinism holds** — the seat checked every `HashMap` in the fixpoint and
confirmed none is ever iterated. And the reach predicate and the emitter's range check are **the same
arithmetic expression** for all three PC-relative kinds, so a mis-sized branch cannot be silent.

The gap is coverage: **of eight exact reach boundaries, one has a test** (`-128`). Missing:
`PcRel8` +127/+128 and −129; `PcRelDisp16` at ±0x8000; `Z80JrRel8` at ±127/128. The linker's own JR
tests only exercise disp=0 and a wildly-out-of-range target.

Two real hazards it did find:
- **F1 — the AS frontend bakes alignment padding with no provisional wall.** `directive_align`
  (`sigil-frontend-as/src/eval.rs:2699`) computes pad from `here()` and emits a fixed `Fill`, while
  the same frontend emits length-variable `JmpJsrSym` fragments. The `.emp` frontend closes this by
  *refusing* (`lower/mod.rs:984`); AS has no such wall. Bounded in practice because 68k growth is
  always even, so `align 2` is invariant — coarser aligns in AS ports are exposed. No test.
- **F2 — `Reserve` before a relaxable diverges the reach-test site from the fixup-apply site.**
  `frag_len` counts `Reserve` (VMA space); `link()`'s fixup walk skips it (image space). The
  identical hazard *is* guarded when `Org` and `Reserve` coexist (`relax.rs:611-643`); `Reserve`
  alone is unguarded. A displacement still in range after the shift is written **wrong, silently**.

### S7 — No IR validation pass; invariants are prose enforced three inconsistent ways · MEDIUM
**Seat:** IR

No `validate`/`verify` function takes a `Module` or `Section` anywhere in the workspace. The
invariants are unusually *well documented* but enforced at three sites by three mechanisms —
`unreachable!` panics in release, `debug_assert!`, and `Error` diagnostics — and five stated
invariants are checked by nothing at all.

The sharpest unenforced one: **`IrStreamer::emit_fragment(frag, advance)`** (`backend.rs:76`)
requires `advance` be the abs.w baseline width, because `sigil-link`'s `shift_breakpoints` assumes
exactly that. Stated in `sigil-ir`, relied on in `sigil-link`, satisfied in `sigil-frontend-emp` —
**three crates, zero checks**. A wrong `advance` produces silently wrong label addresses.
`frag_len(frag, 0)` already computes it; `emit_fragment` should derive it rather than accept it.

Also: `BankPtr16Le/Be` does `value as u16` with **no range check**, while the byte-identical
`Value16Le` goes through an unsigned-window check that `lower/data.rs:168` explicitly relies on as
the cross-CPU safety net. And bank semantics aren't in the relocation at all — `winptr`/`bankid` are
constant-folded into generic arithmetic before the linker sees them, so it cannot validate that a
windowed pointer's target is actually in a bankable region.

### S8 — Two crates are 93% of build time, in series; `sigil-harness` is accretion · MEDIUM
**Seat:** ARCH

Clean release build 25.8 s: `sigil-frontend-emp` 15.0 s (58%), `sigil-harness` 9.0 s (35%). They sit
in series on the critical path — `sigil-cli` cannot start compiling until 23.8 s in.

`sigil-harness` holds seven jobs and **680 `pub` items against 4 `pub(crate)`** (170:1; the next
worst crate is 432:160). It straddles test and production: `sigil build` routes the entire shipping
build through `sigil_harness::native`, while the same crate exports `pins` (generated), `repin`,
`test_support` and `provenance` — all of which compile into the rlib the shipped binary links.
Editing a test fixture rebuilds a crate the binary depends on.

Three clean seams, in value order: `sigil-build` ← `native`+`map_placement`+`contract_baseline`+
`seam1/2` (~7,400 LOC); `sigil-pins` ← the generated `pins.rs`; `sigil-testkit` ←
`test_support`+`repin`+`provenance` (~1,900 LOC) moved to dev-dependencies. Mostly manifest edits.

Also: **`sigil-salvador-sys` has no upstream commit hash or vendoring date**, and was vendored from a
sibling working tree rather than upstream. The other two `-sys` crates are pinned exemplarily with
commit hashes, dates, per-file tables and vector provenance. A ~20-minute fix with the template
already in the repo, and it is the workspace's weakest reproducibility link — which matters because
the whole golden argument rests on reproducible output.

### S9 — `banked_carriers` PINS the banked head VMAs, with no cross-check against the derivation · HIGH
**Seat:** LINK · **This answers aeon's open question — the answer is "it pins."**

`seam1.rs:138-155` is a hand-maintained `(symbol, VMA)` literal table, injected as `equ` carrier
sections into the seam-1 link (`seam1.rs:492-511`, `:559`, `:578`) that produces the **shipped**
resident Z80 blob (`seam1.rs:640-687` → `boot_data.emp:46-47` embeds it). The literal `0x8571` at
`seam1.rs:144` is baked into the driver's operand bytes. **Nothing derives it.**

**The pins are currently correct; aeon's comment is the stale artifact** — the reverse of what the
aeon packet assumed. Derived truth vs `soundbankhead.emp:14`:

| head | derived VMA | aeon says |
|---|---|---|
| `SoundTablesZ80_Head` | `$8000` | `$8000` ✓ |
| `SndDefaultPitchTable` | `$8357` | `$8357` ✓ |
| `SfxBlobWinTab` | `$845F` | `$845F` ✓ |
| `SeqOpcodeTable` | **`$8571`** | `$856D` ✗ |
| `DacSampleTable` | **`$85B1`** | `$85AD` ✗ |

`seam1.rs:140-143` records the `0x856D → 0x8571` bump when `$BA`/`$BB` widened the win table. Aeon's
line was never updated. Also: `DacSampleTable` is **not** a carrier (`seam1.rs:137` says so) — it is
derived via `seam2::dac_sample_table_vma`. Aeon's list is wrong to include it.

**The finding:** `banked_carriers` is referenced at three sites and **nothing compares it to
`seam2::sound_layout`**, which derives the same addresses correctly. The only backstop is a frozen
golden byte gate — precisely the comparand that gets re-blessed during a size campaign. The next SFX
growth moves the derived head, leaves the literal stale, breaks the gate, and the natural remediation
(refreeze) blesses the wrong blob. `native.rs:2210` already draws this exact lesson: *"a second copy
of this arithmetic is the bug, not the fix — that is the lesson of the three unmaintained copies of
the sound-bank addresses."*

**Fix:** derive the three from `seam2::sound_layout` the way `DacSampleTable` already is
(`seam1.rs:1084-1087`), or add an always-on equality assertion.

### S10 — `[[hole]] at` is inert schema; the real resume address is `$3F8`, not `$3FE` · LOW
**Seat:** LINK · **Answers aeon's second open question: advisory, not enforced.**

`Hole.at` is read at exactly one site — inside a `format!` string (`native.rs:3002`). `filled_by` is
never read at all. So the aeon worry does not materialize. But the numbers confirm the field is dead:
both sound-off shapes place `Z80_IdleProgram` at `0x3d0..0x3f8` (40 bytes), so the map's
`at = 0x3FE` is off by 6 and always has been. Three files carry the stale 38-byte figure
(`sonic4/map.toml:135`, `demo/map.toml:53`, `boot_data.emp:21`); placement is contiguous packing and
absorbed the 38→40 change silently and correctly. **Either enforce `at` or demote it to a comment.**

### S11 — `check_error_handler_is_last()` fires on every shipped build except `--lean` · answered
**Seat:** LINK · **Answers aeon's third open question: yes, with two caveats.**

Definition `native.rs:3692`, single call `native.rs:3738` — the first statement of
`append_deb2_appendix`, before convsym is shelled. Reached by every shipped `sigil build --native`
target via `main.rs:1690`. It checks something *stronger* than "is last":
`appendix_start == ErrorHandlerBlob + 0xF56` exactly, hard `Err` with a signed byte count on any
drift. Caveats: it returns `Ok(())` vacuously when no `ErrorHandlerBlob` symbol exists (correct by
design for the island-less shapes), and `--lean` skips it unconditionally. **aeon's claim at
`error_handler.emp:66` is accurate.**

Also from LINK: `[map.order-undeclared]` — the completeness check the K5 inversion's teeth depend on —
**skips label-less sections** (`native.rs:2977`, `if id.is_empty() … continue`). Label-less
byte-emitting blobs are a supported, documented case, so an unnamed emitter is placed by contiguity
and never order-validated. And `RegionKind::Z80Bank`/`M68kRam` are parsed and never read — sonic4's
`z80_moving_trucks_bank` declares `0x60000` while the MT bank actually sits at `0x58630`; demo
already deleted its copy with a rationale.

### S12 — `jsr (aN) as Type` narrows the clobber closure on a check with zero producers · CRITICAL
**Seat:** Va · **Arguably worse than the `targets(...)` instance.**

`subcontract_violations` (`closure.rs:328`) is documented as "what makes a dispatch target
installable" and has exactly two call sites — both in interface `implement` binding
(`resolve/contract.rs:416`). **It is never called on a dispatch site.** Meanwhile `closure.rs:232`
*consumes* the `as Type` bound to replace ⊤ with the type's clobber set.

Unlike `targets(...)` — which sigil's own doc explicitly quarantines to the opt-in budget
(`cycle_budget.rs:653`: "can corrupt no soundness-bearing analysis") — `as Type` feeds
`check_firings`, `check_live_clobbered`, **and `preserves::find_dead_saves`**, whose output is a
*deletion instruction to a human*. So a wrong narrowing can cause the checker to recommend deleting
the guard that made the code safe.

**Live in aeon at 8 sites** (`core.emp:519,571`, `collision.emp:78`, `player_common.emp:639,1011,1015`,
`player_sensors.emp:266`, `characters.emp:151`). Aeon already knows: `core.emp:520` ships a DEBUG-only
**runtime** assert commented "*catch object routines violating the a0/d7 preservation contract*" — a
runtime workaround for a compile-time check the type syntax implies exists.

**Fix:** either wire `subcontract_violations` at the `as Type` site, or stop letting the bound narrow
the closure until it can. The current state is the worst of both.

Va's other high-value items: **`indirect_sites` reads the raw AST while `local_writes`/`direct_callees`
read the post-comptime CodeBuf** (`corpus_contracts.rs:2189` vs `:2162`), so an indirect call emitted
from a `comptime fn` or a splice contributes *no site at all* — the ⊤ that is "§1's load-bearing fact"
silently never fires (latent; no live aeon instance, and no test). **A `preserves` deferral on a proc
with no `clobbers` clause has no authority on either side of the seam** (`closure.rs:428` skips
`!has_clobber_contract`), contradicting `lower/proc.rs:1783`'s "nothing silently ships."
**Interface/hook signature checking is vacuous whenever the bound proc omits `clobbers`**
(`resolve/contract.rs:470` — `p.clobbers.as_ref()?` returns `None` and the whole check is skipped, so
removing a clause makes a proc satisfy *every* signature). And **three error-tier analyses are computed
on every build and gated by nothing** — `branch_const_firings` (named after a real shipped bug),
`bus_firings`, `slot_firings` — they are merely printed.

### S13 — `@scaffolding` guards a lint that was never built · MEDIUM
**Seat:** HALF

The mirror image of `rst`: fully implemented as *syntax* (mandatory reason string, parse-validated),
existing solely to suppress "D7's dead-symbol analysis" (`ast.rs:920`) — **which does not exist
anywhere in the workspace.** Worse than dormant, it is *load-bearing*: `children.emp` carries four
`@scaffolding("…zero call sites today; kept deliberately…")` annotations whose authors believe a lint
is reading them.

Same seat: **`patch`/`bind` parse and are a silent no-op** (`eval/control.rs:166`) — while the actual
mechanism (`lower/patch.rs`, 208 lines with full diagnostics) is built, self-tested, and never wired
to the parsed statements. Pure wiring task; the hard part is done. And a family of complete,
checked, diagnosed features has near-zero adoption: `enum` 0, `bitfield` 0, `dispatch` 0, `script` 0,
`comptime test` 0, `table` 1/142 — mechanically complete, so "unfinished" isn't the explanation;
"undiscoverable or superseded by convention" is the likelier one, and worth a decision before further
investment.

### S14 — The whole-program contract gate has no source location at all · MEDIUM
**Seat:** ERR

The phase most needed on a 142-module build is the worst-located in the tool. `run_contract_gate`
(`sigil-cli/src/main.rs:~1000`) collapses `OutFiring`/`InoutFiring`/`LiveClobberFiring` — all of which
**carry a real `span`** (`out_verify.rs:242`, `calls.rs:131`) — into bare `(proc, reg)` strings before
diffing against the baseline. A new firing's only breadcrumb is a proc name to grep for.

Contrast the best message in the tool, live-verified: an `ensure` failure anchors at the *guard's own
line* with working `{n}` interpolation. And multi-error reporting is correct everywhere — no
first-error-and-stop anywhere in the CLI. Also: cross-module "unknown symbol" anchors at the **module
header, not the reference site**, with a `TODO` in `resolve/mod.rs:906` admitting it.

### S15 — A green CI badge carries zero whole-ROM byte evidence · CRITICAL — read this before trusting any prior sweep
**Seat:** GATE

**Every sweep this week ended some finding with "the whole-ROM byte golden would catch that." That
premise does not hold in CI.**

- `.github/workflows/ci.yml:22-25` sets `AEON_DIR=/nonexistent/aeon-not-checked-out-in-ci` and does
  **not** set `SIGIL_STRICT_GATE`. The comment is candid: *"the skip is what CI exercises."*
- Reproduced in that exact environment: **3,673 tests run, 3,672 pass, and 352 of them (9.6%) skip
  silently green** — 203 in the ROM/aeon-tree-presence class, across 105 test binaries. **Nothing
  counts or reports the skips.**
- All 28 build-and-compare golden gates are in that set: every `native_offcanonical_rom` anchor
  compare, all 9 `native_offcanonical_full`, all 9 placement gates, `native_full_rom`, both
  `native_rom`, `pins_rs_is_current`, `m1b_gate`, `m1c_vector_table`, all 6 `seam1_native_link`.
- What *does* run is `refreeze --check` / `provenance_chain_holds` — which **never builds anything**.
  `do_check` (`refreeze.rs:146-167`) reads `golden/provenance.toml`, reads `golden/*.bin`, and
  recomputes CRCs. Its own doc calls it "the gate." It is two committed files agreeing with each
  other, and it is structurally incapable of answering "does the build produce these bytes?"

That fully explains the `refreeze --check`-green-while-16-golden-tests-red incident: the two gates
have **disjoint subjects**. `m1b_gate.rs:9` states the consequence outright: *"A green GitHub CI does
NOT prove the checksum or listing-fidelity claims."*

**And re-freeze has no intent/regression discriminator.** The sole precondition is
`ab.trim().is_empty() || ab == "asl-witness"` (`refreeze.rs:210`). Any other non-empty string passes.
Of 111 committed chain entries: 67 name a document that resolves on disk, **33 carry free prose
naming no document** (one literally defers its proof to a future session), 11 are empty or the
sentinel. A new target key is never checked at all (no `prev` to compare — `lean` entered this way).
And `--freeze` regenerates `pins.rs` and the size tables *first*, then reads `anchor_end` back out of
them — **the measurement window is redefined by the artifact being measured.**

**Plus a confirmed vacuous pin.** 76 of 95 region pins run `[module_A_head, module_B_head)`, so the
window includes inter-module placer fill; 49 end in zero bytes. `OJZ_BG_ANIM` (`pins.rs:253`) is a
14-byte all-zero plain window compared against a module the test zero-pads to 14 bytes — **it passes
if `bg_anim.emp` emits nothing at all.** 39 test files carry an explicit "tolerate a `< 16 B`
all-zero tail" escape hatch that silently absorbs module shrinkage.

**Consequence for the three aeon packets:** any finding whose only detector was "the golden would
move" is weaker than recorded — the golden does not run in CI, and a re-freeze is a one-command
operation gated on a human typing a non-empty string. This does not invalidate those findings; it
invalidates the *reassurance* attached to several of them.

### S16 — 22 sigil↔aeon seam mirrors; 4 already diverged, 14 unguarded · HIGH
**Seat:** B2

The seam nothing can check — different languages, different repos. All 22 hand-diffed.

**Already diverged:** sigil's `Act` struct fixture is **6 bytes and 2 fields stale**
(`test_support.rs:113` says 34 bytes/`$22`; `engine/structs.emp:26` says 40/`$28` — and aeon's own
line 1 still says 34, so the stale number has two homes). Consumed by 12 port gates. Currently inert
only because no live `.emp` externs `Act_len`.

Plus the Z80 idle 3-way (S10 above), `CONVSYM_FILTER` citing a `build.sh` line that is now a
different command entirely, and `build.sh:229` contradicting `build.sh:11-15` on whether release
ships the appendix.

**And a bulk defect: 190 dangling `.asm`/`.inc` citations across 70 sigil files**, naming 14 aeon
files the flip deleted (`engine/constants.asm`, `structs.asm`, `main.asm`, `engine.inc`, …), many
with exact line numbers that are doubly dead. Every one is a hand-copied constant whose authority no
longer exists — that is the mechanism by which the `Act` and Z80-idle drifts happened and will
happen again.

**The standard to copy is already in the repo:** `pins.rs` + `repin_pins.rs` (generated from a live
resolve, with a staleness gate), `validate_placement` (bidirectional), `check_error_handler_is_last`
(a mirror that checks itself), and `seam1::resolve_consts` — *"No value is hand-maintained here"*, it
parses the values out of aeon's own `.emp`. That last one is exactly the mechanism the `Act` fixture
needs.

### S17 — CORRECTION to S2: the build is 2.8 s, not 12.7 s, and the root cause is not the threads
**Seats:** P1a (forward), P2 (altitude) — both re-measured independently

**S2 above is wrong and this supersedes it.** P1b measured a 4.3 s CPU / 12.7 s wall build. P1a and P2
each re-measured and found **that number was machine contention from this very lens panel** — 11
subagents were hammering a 16-core box at load 9-18. On a quiet box, three runs each:

**2.85 s wall, 2.1 s user + 0.7 s system, single-threaded, for a 696,836-byte ROM from 43,467 lines
of `.emp`.** That is ~15k lines/s. **Sigil is not a slow assembler.** P2 states it plainly: the
correct read is not "the pipeline is wrong," it's "the pipeline is right and has never been asked to
use more than one core."

This is a process lesson as much as a perf one: **the panel measured itself.** Any wall-clock number
taken during a parallel sweep needs an `uptime` check beside it.

**The real #1 defect, and it is still large.** P1a instrumented the binary via gdb (ptrace blocked, so
non-stopping Python breakpoints at phase boundaries) and got a phase profile plus exact call counts:

| what | count |
|---|---|
| `resolve::ambient_items` | **3.14 s of 3.18 s** of `build_program_with` (86% of build) |
| `fold_const_literal` / `eval_const_with_root` | **36,133 calls** |
| distinct `pub const` definitions in the corpus | **853** |
| `stamp_overlay_window` | 43,754 |
| `parse_file` | 540 (142 distinct files — 3.8 full corpus re-parses) |

**Sigil evaluates ~853 distinct constants 36,133 times — a 42× redundancy factor.** `collect_pub_comptime`
clones every `pub` comptime item into every consuming module, and folds each one *per consumer*: M
defining consts × N consuming modules. Each fold spawns a fresh 64 MiB-stack OS thread, builds a
fresh `Evaluator`, and **re-indexes the entire defining file** (113 KB of AST for `sound_sequencer.emp`)
to resolve one integer, then throws it all away. Measured 63 µs per call, of which 19 µs is kernel
time — so **~25% of the whole build is the kernel managing 36k short-lived 64 MiB-stack threads.**

**The fix already exists in the same module.** `eval::eval_all_pub_consts` (`eval/mod.rs:1668`) builds
*one* evaluator per file and folds every `pub const` through a memo. Calling it lazily per defining
module collapses 36,133 evaluator constructions to **≤142**. P1a's estimate: **2.85 s → ~0.7 s, a
3.5-4× speedup on every build.** Risk LOW — pure-function memoization, and the golden CRC gates
verify byte-identity.

The code already knows: `resolve/mod.rs:105` carries a `TODO(perf)` about this exact M×N shape — for
*overlays*, which never fire in a real build. The same defect on *consts*, which is 36,133× hot, is
unremarked.

**Two measured non-findings worth recording so nobody spends effort there:**
- **LTO + `codegen-units=1` is worth 0%.** P1a built a second binary with both and timed it:
  2.10/2.13/2.20 s vs 2.11/2.14/2.22 s baseline, byte-identical output, for 54 s of added toolchain
  time. This is the most commonly-recommended "free" assembler speedup and it is measurably worthless
  here.
- **The relaxation fixpoint is 2% of the build.** P2: "ranking this above the const fold would have
  been the classic mistake — it is the ugliest code in the pipeline and nearly the coldest."
  `symbols.rs:56`'s `to_string()`-per-lookup is a two-line free win; the incremental-symbol-table
  rework is not worth the risk at 4,264 symbols.

**And P2's explicit DON'T:** a module→object incremental cache would be **negative today**. The cache
key is nearly the whole program (ambient items + region_ends + iface_env + defines + every `embed()`ed
binary), the hit rate on a real edit would be poor, and — the real argument — a stale entry doesn't
produce a compile error, it produces **a wrong ROM that passes**. Given S15 above, that is precisely
the failure mode the pin apparatus exists to prevent. Parallelism is invisible to correctness because
the goldens prove it bit-for-bit; caching is not.

### S18 — A 7-byte input hangs the build forever · HIGH
**Seat:** FUZZ

Built a mutation fuzzer over 2,738 real `.emp` files, 20,000 trials. **Zero panics** — genuinely good.
But six hangs, four of which reproduced standalone against the release binary, and delta-debugging
minimized the smallest to **seven bytes**:

```
printf 'context' > t.emp && sigil parse t.emp     # hangs forever, no diagnostic
```

`item()` accepts `context` only when followed by an identifier. `recover_to_next_decl` lists `context`
among its recovery openers and stops there **without consuming it**. So `item()` fails on the same
token, recovery stops on the same token, forever — zero-progress infinite loop.

**This is the same bug class already fixed once, for `extern`.** `parser_recovery_hang.rs` exists
specifically for it and its header says *"A front-end must error loudly, never spin."* The fix was
applied to `ensure`/`align`/`extern`/`interface`/`implement` and **never generalized to `context`**,
its structurally identical sibling. FUZZ verified every other contextual opener terminates correctly;
`context` is uniquely broken. One-line fix mirroring the existing guard.

### S19 — Two confirmed stack-overflow aborts; `unsafe` outside the FFI crates is zero · HIGH
**Seat:** SAFE

**The good news first, because it's the headline:** `unsafe` outside the three vendored `-sys` crates
is **zero** — the one grep hit is the word inside a doc comment. The from-scratch-safe-Rust claim
holds. Default clippy is clean of anything in the panic/overflow/unsafe class.

Two confirmed **uncatchable process aborts** (SIGABRT, no diagnostic — a stack-overflow signal can't
be `catch_unwind`'d, so `sigil build` just dies):

1. **`sigil-frontend-as`'s expression parser has no depth guard at all.** A syntactically *perfect*
   `dc.b (((...1...)))` at 40,000 nesting levels aborts. Reachable from every operand and every
   `dc.b/w/l` — and this frontend is still live, assembling `game_root.asm` on every build.
2. **`sigil-frontend-emp`'s depth guard is real but its documented completeness claim is false.**
   `parser.rs:3894` claims `primary_expr`/`unary_expr`/`ty` "cover every unbounded recursion path."
   They don't: `postfix_expr`'s bracket-index arm recurses into a fresh un-gated `self.expr()` without
   incrementing depth. When the 128-guard fires it returns poison *without consuming the `[`*, so the
   enclosing frame re-reads it as an index and starts another 128-deep descent. **Stack depth grows
   linearly with input, not capped at 128.** Confirmed in gdb at 2,600+ alternating frames. The paren
   equivalent at 200,000 levels does *not* crash — the control that proves the mechanism.

Also: `sigil-salvador-sys::compress` is the one FFI wrapper that `assert!`s instead of returning
`Result`, so a C-side failure can never become a diagnostic. Its two siblings are exemplary — and
`clownlzss-sys` documents a **real past heap overflow** (~50 bytes) fixed by adding a C-shim capacity
check. Given that history, the absence of any fuzz harness over the three compression crates is the
gap SAFE flags.

### S20 — The suite's own numbers · HIGH corroboration
**Seats:** TEST, A2

TEST independently reproduced GATE's CI finding by a different route and landed on the same shape:
**3,676 tests, 3,672 "pass", of which 360 are silent runtime skips** that print `skip:` and return
without executing an assertion — invisible because CI doesn't pass `--nocapture`. Genuinely-exercised:
**3,312.** Four `#[ignore]`d tests, three marked RETIRED; **nothing anywhere runs `--ignored`**, so
`sigil diff` is never exercised automatically at all.

TEST also names the self-fulfilling-fixture problem precisely: `native_rom.rs` still carries the
docstring *"prove the whole ROM equals the live `asl` s4.bin"* — **that claim is now false**, the
comparand is sigil-built. The README status table still says "byte-identical to `asl`." The honest
mitigating nuance: the ISA-level golden vectors *are* frozen from a real pre-flip `asl` run and remain
valid, and many `*_port` tests are differential between sigil's two frontends. The circularity is
specific to the whole-ROM gates — which are exactly the ones that don't run in CI.

A2's doc-truth sweep found ~65-70% true overall, but with a different *shape* than the aeon sweeps:
the class that matters most — "what a check proves" — sigil gets right almost without exception
(5/5, test-corroborated). What rots is **headline numbers written at a landing commit and never
revisited**: `main.rs:16` still describes a `main.asm` include tree that doesn't exist, `native.rs:232`
says "the 52 modules" when it's 84/95, and the README's "four header bytes" was abandoned by the
codebase itself five days later in favour of dynamic derivation. Four of five findings trace to just
two large prose-heavy landings (2026-07-30 flip, 2026-08-04 crash-report ruling).

### S21 — `ensure` reachability is a silent shape rule; ~14 shipped guards never evaluate · HIGH
**Seat:** COMPTIME

**The evaluator's arithmetic is not the bug** — bare `Int` comptime math is `i128` + `checked_*` and
errors on overflow; div/mod by zero error; shifts are range-checked. The vacuous guards the aeon
sweeps found are almost all *reachability* bugs.

An item-position `ensure` is evaluated **iff its module is lowered**, and a module is lowered iff it's
in the `use`-reachability closure of a synthetic entry seeded from the profile's registry. **25 of 142
modules are outside the sonic4 closure**, and there is **no `[module.unreachable]` lint anywhere** —
no warning, no report mode. The only way to discover the rule is to read `native.rs`.

Most of the 25 are covered by other paths (seam1/seam2 lower the sound modules). Four are not:
`engine.z80_init` (1 guard — **confirms the prior finding, with the exact mechanism**: it's in the
registry only for `demo` and `config_b`, so both shipped sonic4 shapes never evaluate
`ensure(extern("Z80_IDLE_SIZE") == 40)` and never even *defer* it), `sound_debug` (2),
`compression_selftest` (2), `games.demo.constants` (9).

One lint after the BFS — *"module X has N ensure(s) this profile never evaluates"* — retires the whole
class. COMPTIME also found the **one typed-arithmetic trap**: `Value::Typed` ops use `wrapping_*` +
width-truncation, and a bare literal **coerces into the typed operand's width**. So on an unsigned
newtype `ensure(A - B >= 0)` is *always true*. Eleven live `VramTile` guards sit on that surface;
none is vacuous today (all operands are small), but it is armed and nothing flags it.

And two aeon comments claiming "a comptime span primitive awaits implementation" are **stale** —
`span()` does exactly what they ask and both sites are pure-data procs in its domain. A third
complaint survives scrutiny: there is genuinely no *section*-length primitive.

---

## 3. Recommended order for the implementing agent

1. **S3 (thread spawns)** — biggest win, self-contained, changes the answer to the incrementality
   question before anyone spends effort on it.
2. **S1 (EA-class validation)** — the correctness fix. Add `ea_class` + per-mnemonic `EaClassSet` in
   `sigil-isa`, make it exhaustive so new mnemonics fail to compile until classified. This closes S1
   and S2 together. **Add the negative tests in the same parcel** — the oracle cannot generate them,
   so they must be hand-written.
3. **S4 (Z80 clobbers)** — either implement the undeclared-write check for Z80, or correct the two
   aeon headers to say what is actually verified. Do not leave the claim standing.
4. **S7's `emit_fragment` derive** — one-line structural fix for a three-crate unchecked contract.
5. **S6's boundary tests** — seven missing reach-boundary cases, cheap.
6. **S8's harness split** — mostly `Cargo.toml`; takes ~1,900 LOC off the shipped binary and the
   rebuild path.
7. **S5's `rst`** — scoped at ~20-40 lines; unblocks ~116 B in the sound driver.
8. **README + salvador provenance** — cheapest items in the packet.

---

## 4. Cross-finding note for whoever owns build performance

S3 changes the incrementality calculus. A full build at ~4.3 s is fast enough that a caching layer
may not be worth its risk — and a stale-cache bug in a build tool is far worse than a slow build,
particularly here where the golden/pin machinery depends on whole-build determinism. **Land S3
first, re-measure, and only then decide.** The seat assigned that question (CACHE) had not reported
when this packet was written.

---

## 5. Verified-clean — recorded so it is not re-litigated

- **The crate graph is a clean 6-layer DAG**, no cycles, no back-edges, **machine-enforced** by
  `crates/sigil-cli/tests/crate_graph.rs` (380 lines, hand-rolled JSON parse, zero external deps,
  with explicit non-vacuity checks). `sigil-isa` has zero workspace deps by design and is reached
  only through backend facades, keeping it extraction-ready.
- **Zero feature flags** — no `[features]`, no `optional = true`, no `#[cfg(feature)]` anywhere.
  Build shapes are runtime `GameProfile` values, so every shape is reachable by a test without a
  rebuild. Strictly better than cargo features for this problem.
- **The IR is genuinely target-neutral** — because it sits *below* where target differences live.
  There are no instruction nodes at all: frontends encode, the IR carries bytes and relocations.
  `rung_reaches` derives reach purely from `FixupKind`, and that generalization was proven by a
  second target (Z80's `jr → jp` ladder reuses it unchanged).
- **The Z80 T-state table is trustworthy.** Hand-verified entry by entry against the Zilog reference,
  including all four taken/not-taken conditional splits. Single-sourced (`instr_cost`), consumed by
  both `cycles()` and `@budget`. Unknown forms bail loudly rather than guessing.
- **Relaxation termination is proven and determinism holds** (see S6).
- **The Z80 backend fails closed, not open** — CGb found no form where sigil emits wrong bytes for
  something it claims to support.
- **Fixup-kind precision is exemplary** — every width-1/width-2 split (`ImmSigned8` vs `Value8`,
  `ImmWord16Be` vs `Value16Be` vs `Abs16Be`) is documented against a concrete mis-assembly it
  prevents. Precision earned from probing real `asl`.
- **The frontend/IR expressiveness gap is caught at the frontend seam** with named lint ids
  (`[lower.abs-sym-operand]`, `[cross-cpu.unwindowed-pointer]`, `[branch.non-68k]`) — the correct
  answer to the half-implemented-feature problem, and this codebase has it.
- **Two of three vendored C dependencies are pinned exemplarily** (commit hash, date, per-file
  upstream table, license reasoning, documented wrapper contracts).

## 6. Known gaps in this packet

Beyond the sixteen unreported seats listed at the top: no seat ran a differential against a reference
assembler, because none is installed (`asl` was removed from the repo at the flip). CGa worked around
this by decoding sigil's computed opcode words with Capstone 5.0.7 in `CS_MODE_M68K_000` — a real
oracle for the alias findings, but an inverse one. Installing `vasm` or `asl` and running a genuine
differential corpus remains the highest-value untaken verification step for the 68k backend.
