# 2026-07-29 — Z80 rung-2 contract vocabulary + relaxation ladder (DESIGN DRAFT)

Status: **DRAFT for overseer review** (Fable session, drafter deliverable). Written but
NOT committed, no other file touched — the T1 flow (`2026-07-28-z80-t1-operand-model.md`
was drafted, reviewed, then authorized). This note is the design input a rung-2 tranche
brief is cut from; it does not itself land code.

Sibling to the recon (`2026-07-28-z80-recon-emp-design.md`, §4.2/§5 rung 2) and the T1
note (`2026-07-28-z80-t1-operand-model.md`, whose §0 RUNG-2 TEST OBLIGATION and §6
interface points this note discharges). RULED context in force: recon §6 ruling 3 —
module-scope `invariant` is a NEW CONTRACT CLASS, surface lands with the rung-2 files that
demand it, TDD'd. Ruling 2 — comptime T-state accounting is BUILD-later (rung 4),
straight-line scope; §8 names only its seam.

**Scope = the two rung-2 files, read as the demand corpus** (like T1's satellites):
`engine/sound/sound_psg.asm` (526 L, per-routine clobber/preserve headers) then
`engine/sound/sound_fm.asm` (998 L, invariant-heavy). Everything here is designed against
what those two files actually spell; representations are defined whole so later rungs are
additions, but only demanded forms are wired and tested.

Expected byte movement: **ZERO**. The whole rung is sigil-side (new contract checker, new
module attribute, new tests) plus byte-locked ports of psg/fm against asl. Blob precedes
engine — any nonzero delta is the STOP condition, never an absorb (the t25 rule).

---

## 0. Charter in one line

T1 gave `.emp` a Z80 *operand* model. Rung 2 gives it a Z80 *contract* model: the register
contract vocabulary (`clobbers`/`preserves`/`out` on Z80 registers), the module-scope
`invariant` class, and the checker that replaces the t27 corpus-contract SKIP for
`(cpu: z80)` modules — scoped to what psg/fm demand, proven by porting them byte-identically.

---

## 1. What the rung-2 corpus actually demands (scope = the two files)

### 1.1 The register-contract surface (both files, every routine)

psg/fm carry an accurate per-routine `Clobbers:`/`Preserves:` header (psg header line 60:
"a false clobber comment caused a prior bug" — the exact footgun the checker closes). The
demanded contract forms, mapped to the existing trichotomy (spec D2.7, line 107: `clobbers`
/ `preserves` / `out`):

| Form | Corpus site | Notes |
|---|---|---|
| `clobbers` on Z80 regs | `sound_psg.asm:68` `Clobbers: af` | 8-bit + pair names |
| `clobbers` a register HALF | `sound_psg.asm:91` `Clobbers: af, b` / `Preserves: c` | `b` clobbered, `c` preserved — pair SPLIT |
| `preserves` on Z80 regs | `sound_fm.asm:57` `Preserves bc, de, hl, ix` | via push/pop or never-written |
| `out` (single reg) | `sound_psg.asm:67` `Out: a`; `sound_fm.asm:88` `Out: b`,`c` | result register |
| `out(carry: …)` flag result | `sound_psg.asm:120` `Out: carry clear … / carry set on unknown id` | condition-code result |
| register-param inputs | `sound_fm.asm:631` `In: a = note, c = max, ix = SeqChannel` | typed proc params (§2.3) |

The register UNIVERSE for Z80 contracts is finer than 68k's `d0..a7`: it is the 8-bit
halves `{a,f,b,c,d,e,h,l}` plus the index/pair units `{ix,iy}` (sp is stack discipline,
the a7 analog; `i`/`r` are rung-1 only and never appear in psg/fm contracts). A pair name
in a reglist EXPANDS to its halves: `bc→{b,c}`, `de→{d,e}`, `hl→{h,l}`, `af→{a,f}`. This is
the whole reason `clobbers(af, b)` + `preserves(c, …)` is expressible (psg
`Psg_VolToAtten`): `b` and `c` are independent units. 68k has no register-half analog — so
the Z80 reglist vocabulary is a genuine widening, not a rename.

### 1.2 push/pop as the save mechanism (the checker's core new job)

68k proves `preserves` by movem/`move.l aN,-(sp)` pair-equality (spec D2.32; §5 of
`preserves.rs`). Z80 has no movem — every save is a `push rr` / `pop rr` pair. The corpus is
dense with them (both nested and around clobbering calls):

- `sound_psg.asm:320-324` — `push de` / `call Psg_ChBase` / `pop de` (save `de` across a
  callee that clobbers it).
- `sound_psg.asm:394-396`, `:438-440` — `push hl` / `call Snd_ChanClass` / `pop hl` (the
  routine `preserves hl` by bracketing the CALL that clobbers it — TRANSITIVE clobber, the
  Z80 analog of the 68k closure's job).
- `sound_fm.asm:219-239`, `:265-331`, `:463-529` — deeply nested LIFO `push bc / push de /
  push hl … pop hl … pop de … pop bc` inside per-operator loops.

### 1.3 The `push R / pop R'` register-MOVE idiom (NOT a save)

`push ix / pop hl` copies ix→hl. It appears at `sound_fm.asm:122-123` (`Snd_ChanClass`),
`:278-279`, `:481-482`. This is NOT a save/restore bracket — it WRITES hl (an out/clobber)
and leaves ix untouched. The checker must read `push R1 / pop R2` (R1≠R2) as a stack-neutral
move that clobbers R2 and preserves R1, and only `push R / pop R` (same reg) as a preserve
bracket. §4.2 shows the existing slot machinery already gets this right for free.

### 1.4 The standing invariants (the module-`invariant` demand)

Enumerated exactly from the two file headers (§3 designs the surface; the enumeration is
the demand evidence, and where the tree contradicts the recon it is flagged in §9):

- **`ix` preserved by EVERY routine, both files.** psg header `:57-61`: "Every routine here
  PRESERVES ix … none of them touch ix at all." Every fm `Preserves` line lists `ix`
  (`:57`, `:113`, `:342`, …). No psg/fm instruction writes ix — it is only read as
  `(ix+d)`. This is a CLEAN module invariant: `invariant(ix)`, an implicit `preserves(ix)`
  the checker inherits onto every proc, never broken, never re-established.
- **`de = $4001`** (the DAC-loop's port cursor). This is the messy one. psg CLOBBERS de
  (`:18-21`: PSG loads the divisor-table base into de; "the DAC loop's de=$4001 invariant
  is re-established by the Timer-A tick CALLER … NOT by PSG code preserving de"). fm also
  clobbers de in several routines (`Fm_PatchPtr` `:113` `Clobbers: af, de, hl`;
  `Fm_SetVolume` `:342`; `Fm_PatchLoad` `:159`). The invariant is maintained by the
  DAC/idle loop's push/pop bracket around the WHOLE `Sequencer_Frame` (`sound_fm.asm:13-15`)
  plus fm's absolute-YM-addressing discipline (never loading a port addr into de) — NOT by
  a per-proc or per-module preserve. See §9-A: de=$4001 does not fit "implicit preserves
  unless re-established"; it is a caller-bracketed, driver-lifetime invariant that belongs
  to rung 4, not a rung-2 module invariant.
- **`$2A` parked** (`sound_fm.asm:15-19`, `Fm_ReparkDac` `:81-84`). Maintained by
  RE-ESTABLISHMENT: every multi-write YM batch ends with `Fm_ReparkDac` re-selecting reg
  $2A on $4000. This is exactly ruling 3's "unless it re-establishes" case — but it
  re-establishes a HARDWARE-PORT state, not a register value (§3.3, §9-B).
- **`$2B` never written** (`sound_fm.asm:18-19`). A hardware-register-never-touched rule,
  not a register contract — the analog of the `[bus.*]` VDP lint, out of core scope (§8).

### 1.5 What the rung-2 files DO NOT demand (checked, per charter item c)

Grepped both files for the CPU-specific contract classes the recon §5 lists under rung 2:

- **`di`/`ei`/IFF interrupt state**: ZERO occurrences in psg/fm (the only hit is a comment
  in fm `:713`). The di-whole-sample discipline is the driver top file (rung 4). The di/ei
  lattice is therefore NOT designed here — §8 names it for rung 4. (Charter item c: "check;
  do not invent" — checked, absent, not invented.)
- **Shadow set `exx`/`ex af,af'`/`af'`/`hl'`**: ZERO occurrences in psg/fm. The shadow-bank
  ROM-length hold is the driver streaming loop (rung 4). Shadow-swap vocabulary is
  *represented* (§4.3) so the preserves proof is sound the day it appears, but NOT wired or
  tested in rung 2.
- **The `ex (sp),hl / ret` computed-jump trampoline**: lives in `sound_sequencer.asm`
  (rung 3), not psg/fm. It is the preserve-proof's known alias hazard (§4.2 bailout); named
  for rung 3, not a rung-2 wiring.

This is a real scope correction to the recon (§9-C): rung 2's ONLY new checker demand is
**push/pop preserves + the `invariant(ix)` inheritance**. Shadow/di-ei/trampoline are
later rungs.

### 1.6 Operand forms rung 2 wires (T1 defined the representation; these are the demand)

T1 landed the enum whole (`value.rs:520-550`) and wired only rung-1 forms. Rung 2 wires the
ones psg/fm spell, each a one-arm addition:

- `(ix+d)` — pervasive (`(ix+sc_route)`, `(ix+sc_volume)`, …). `Z80Indexed`
  (`value.rs:530`), i8 disp check reused from the 68k brief path (T1 §3). Every field
  offset used is < 128 (SeqChannel 60 B / SfxChannel 68 B), so the struct-field ≤+127
  guard (recon §4.5) never fires here — but the check is wired because a future struct
  growth would silently corrupt `(ix+d)` without it.
- `(nn)` absolute memory — `ld (SND_Z80_PSG),a` (const addr → `Z80Mem`), `ld hl,
  (Snd_PitchTabPtr)` / `ld de,(SND_SEQ_PATCHTAB)` (symbolic → link-time mem operand).
- condition codes — `jr z/nz/c/nc`, `ret c/z`, `call nz` (`Z80Cc`, `value.rs:548`).
- `call <label>` / `jp <label>` (tail) / `djnz` — control flow (§5).

`(bc)`/`(de)` indirect, `(ix+d)` with bit ops, and bit-number operands (`bit n,r`) —
psg/fm use `bit 7,d` / `set SCF_KEYED_B,(ix+sc_flags)` / `res`, so `Z80Bit` (`value.rs:550`)
IS demanded by rung 2 (`bit 4,a` `sound_psg.asm:424`; `bit 7,h` `sound_fm.asm:296`;
`set/res` on `(ix+flags)`). Wire `Z80Bit` + `bit`/`set`/`res` here.

---

## 2. (a) Z80 register naming in `clobbers`/`preserves`/`out`

### Options

- **Option A — one CPU-aware reglist recognizer.** `expand_reglist_regs`
  (`lower/proc.rs:1122`) and `Reg::from_name` (the 68k parser) gain a Z80 sibling; the proc
  binder consults the section/module CPU to pick which register vocabulary a reglist is
  parsed against.
- **Option B — a unified register enum spanning both CPUs.** One `Reg` type with `d0..a7`
  AND `a/b/…/ix`.
- **Option C — keep 68k `Reg`; add a parallel `Z80RegUnit` and a parallel reglist path.**

### Argument

Option B repeats the exact mistake T1 rejected for `Value::Reg` (T1 §2.2): 68k `d0..a7` and
Z80 `a/hl/ix` are disjoint universes; unifying churns the frozen 68k contract closure and
buys nothing (a section is one CPU — a reglist is never mixed). Rejected, same reasoning T1
already ratified.

Option A vs C is the reuse-vs-duplicate call. The reglist STRUCTURE is identical across
CPUs (a set of register-unit names, expanded from pair sugar, complemented against the
universe). Only two things differ: the name→unit map, and the universe. So the honest
shape is Option A with a `RegisterFile` seam: `expand_reglist_regs` becomes
`expand_reglist_regs(segs, cpu)` (or takes a small `&dyn RegisterFile`), and the Z80 arm
supplies `{a,f,b,c,d,e,h,l,ix,iy}` + the pair-expansion table. The T1 `Z80Reg8::from_name`
/ `Z80Pair::from_name` (`value.rs:778`, `:829`) are exactly the name recognizers this needs
— already landed, reused, not reinvented.

**Chosen: Option A, register-file-parameterized reglist + universe.** Matches the T1
mirror discipline (the emp contract layer is CPU-parametric, ISA-free) and the campaign's
reuse taste (the same seam feeds §4's checker).

### 2.1 Range form does NOT apply to Z80

The step-2 house rule "contract reglists in movem-RANGE form (`clobbers(d0-d7/a0-a4)`)"
(port-loop §Step 2 item 5) is a 68k-ORDINAL fact: `d0-d7` is a contiguous movem mask.
Z80 registers are not ordinal (no `b-l` range), so Z80 reglists are comma-enumerated:
`preserves(bc, de, hl, ix)`, `clobbers(af, b)`. This is a Z80 divergence from the step-2
range rule — the rule is 68k-scoped and the packet says so (a noticing-clause addition to
the step-2 checklist: "Z80 reglists enumerate; no range form").

### 2.2 The `out` and flag-result contracts

`out` on Z80 registers (`out(a)`, `out(hl)`, `out(b, c)`) rides the exact same trichotomy
machinery — the parser already accepts an `out` reglist (`corpus_contracts.rs:730`); it
just needs the CPU-aware recognizer from §2. The `out(carry: found)` flag result
(`PsgVolEnv_Resolve` `:120`, `FmVolEnv_Resolve` `:150`) reuses the existing `FlagResult`
machinery (`corpus_contracts.rs:590` `flags_of`, the `flag_callees` map) with Z80 flag
names `{carry, zero, sign, parity}` in place of the 68k CCR flags. Callers consume it as
`jr z/c` on the return (`:130`, `:159`) — the caller-side flag check (`check_flag_unused` /
`check_result_invalid_path`) is CPU-agnostic over `CodeItem::Instr` and needs only the Z80
condition-mnemonic recognition T1's `Z80Cc` already carries.

### 2.3 Typed proc register params — the T1 §3.1 deferral lands here

T1 deferred typed register params to "the first rung-2 proc that takes a register param."
`Fm_TransposeClamp` (`sound_fm.asm:631`, `In: a = note, c = max, ix = SeqChannel`) is that
proc. Its `.emp` header binds Z80 registers to types:

```
proc Fm_TransposeClamp (a: u8, c: u8, ix: *SeqChannel) out(hl: u16)
    preserves(bc, ix)  clobbers(af, de, hl)
```

The binder (`reg_from_name` in the proc-param path) becomes CPU-parameterized via the same
§2 seam: under a Z80 module it consults the Z80 map, so `ix` evaluates to a `Value::Z80Reg`
(`value.rs:124`) usable in `{ix}` splices. **This is what produces the source-level
`Value::Z80Reg` the T1 §0 rung-2 test obligation needs** (§6).

### 2.4 Shadow registers in contracts — represented, not wired

`clobbers(hl')` / a `shadow` group are the natural spelling for the driver's `hl'` ROM-length
hold. NOT demanded by rung 2 (§1.5). The representation is reserved in the reglist grammar
(a `'`-suffixed unit, or a `shadow(…)` sub-clause) but neither wired nor tested until the
rung-4 driver top file spells it; §4.3 states how the preserve proof will read it.

---

## 3. (b) Module-scope `invariant` (ruling 3)

### 3.1 Grammar — an attribute on the module header, beside `cpu:`

T1 §5 opened the module attribute list (`ModuleDecl.attrs: Vec<(String, Expr)>`,
`ast.rs:66`) and named it "the ONE forward-compatible slot ruling 3's `invariant(de)` will
attach to." Rung 2 adds the `invariant` key:

```
module psg_writer in z80_resident (cpu: z80, invariant: preserves(ix))
```

`invariant` takes a contract clause (`preserves(...)` today; the grammar is the same clause
parser §2 already builds). Multiple invariants comma-join inside one `invariant: …` or
repeat the key. Reusing the attribute list (not a new top-level statement) keeps the parser
delta to one recognized key and rides the coherence machinery T1 already built
(`[module.cpu-mismatch]`).

### 3.2 Inheritance semantics

Every proc in the module inherits the module invariant as an implicit contract clause,
UNIONED with the proc's own contract. `invariant: preserves(ix)` means every proc is
checked as if it wrote `preserves(ix)` — even procs whose header omits it. The checker
(§4) proves the inherited `preserves(ix)` exactly as a written one: on every path, ix holds
its entry value (never written, or push/pop-bracketed). For psg/fm this is trivially
satisfied (no instruction writes ix) — and that triviality IS the value: the day a future
edit adds `pop ix` or `ld ixl,…` without a matching save, the inherited invariant fires
`[proc.preserves-unverifiable]` on that proc, catching the precise class the psg header
line 60 says already caused a bug once by comment-drift.

### 3.3 Re-establish semantics

A proc may DECLARE it breaks an invariant and relies on external re-establishment. Two
shapes the corpus shows, and the design call on each:

- **`$2A` parked** — re-established at BATCH END within the same proc (`Fm_ReparkDac` tail).
  If `$2A`-parked were a module invariant, a proc satisfies it by ending every YM-write
  batch with the re-park. But `$2A` is a HARDWARE PORT LATCH, not a register — the register
  contract model cannot express "reg $2A on the YM address port holds value $2A." So the
  clean call: `$2A`-parked is NOT a register `invariant`; it is a hardware-write discipline,
  a `[fm.dac-repark]` lint (does every multi-write proc tail into `Fm_ReparkDac`?) in the
  same family as `[bus.*]` — out of the rung-2 core register-contract checker, named for the
  fm hardware-lint follow-on (§8, §9-B).
- **`de = $4001`** — re-established by the CALLER (the Timer-A tick), NOT by the proc. See
  §9-A: this does not fit a MODULE invariant at all (both psg and fm procs clobber de and
  neither re-establishes it). It is a driver-lifetime, caller-bracketed invariant. The
  honest model: `de = $4001` is a rung-4 concern owned by the DAC/idle loop; rung-2's psg/fm
  modules simply `clobbers(de)` where they touch it and carry NO de invariant. The recon's
  "every proc implicitly `preserves(de)` unless it re-establishes" (recon §4.2) is
  contradicted by the tree — flagged §9-A.

**Net rung-2 module invariant: `invariant: preserves(ix)` on BOTH psg and fm.** That is the
one clean, demanded, TDD-able instance. The `de`/`$2A` prose contracts are real but belong
to a different (caller-bracket / hardware-lint) model, not the module-register-invariant
class — a finding, not a gap.

### 3.4 Value-restated bound

`invariant: preserves(ix, de=$4001)` with a VALUE binding (a register pinned to a constant)
is a strictly stronger class than `preserves(ix)` (which only says "unchanged"). psg/fm do
not demand the value form for a MODULE invariant (§9-A moved de out). Represent the
value-binding grammar (`invariant: holds(de == $4001)`) but do not wire it — it is the
rung-4 DAC-loop's spelling, named here so the attribute grammar is forward-compatible.

---

## 4. (c) The Z80 contract checker — what replaces the t27 skip

### 4.1 Where it plugs in

Today `analyze_corpus_with` SKIPS `(cpu: z80)` modules wholesale
(`corpus_contracts.rs:186-189`, `if module_is_z80(&file.module) { continue; }`). The skip
comment (`:180-185`) is precise: the 68k register-contract closure would "drop every Z80
mnemonic as unrecognized," and "Z80 contract analysis (push/pop preserves, shadow sets,
di/ei) is its own rung-2 class." Rung 2 builds that class.

**The 68k closure keeps skipping Z80** (a Z80 proc genuinely carries no 68k register
effect — that half of the skip is correct forever). What changes: instead of `continue`,
a `(cpu: z80)` module routes to a NEW sibling pass, `z80_contracts::analyze_z80_module`,
which returns its OWN firing types (`Z80PreserveFiring`, `Z80InvariantFiring`) folded into
`ContractReport`. The 68k `nodes`/`closure` map stays Z80-free.

### 4.2 push/pop preserves proof — REUSE `preserves.rs`, do not reinvent

`preserves.rs` already does exactly the required shape for 68k (its header, `:1-40`): a
forward dataflow over the CPU-agnostic CFG (`flag_check::Cfg::build(&[CodeItem])`,
`flag_check.rs:157` — no CPU parameter), tracking a symbolic stack of slots each tagged with
which register's entry value it holds, plus a per-register entry-value bit, with soundness
bailouts and `[proc.preserves-unverifiable]` on a written-but-unprovable declared preserve.

The Z80 proof is the SAME dataflow with three substitutions:

1. **Save/restore ops**: 68k's `move.l aN,-(sp)` / `movem` pushes and `(sp)+` pops become
   Z80 `push rr` (push one 2-byte slot tagged with the pair) and `pop rr` (pop + match).
2. **Register file**: the §2 Z80 universe (halves + ix/iy) in place of `d0..a7`.
3. **Alias-hazard bailout**: `ex (sp),hl` and any `(sp+d)`/indexed-sp access is the Z80
   analog of 68k's displaced-sp bailout — the model bails, and a declared preserve over a
   bailed proof is `[proc.preserves-unverifiable]`. (Not in psg/fm; it is the rung-3
   sequencer trampoline. Named so the proof stays sound when rung 3 arrives.)

The `push R / pop R'` MOVE idiom (§1.3) needs ZERO special-casing: push ix stores a slot
tagged "ix's entry value"; pop hl restores hl FROM that slot and sets hl's entry-value bit
ONLY if the slot holds hl's own value — it holds ix's, so hl's bit stays clear (hl
clobbered) while ix's bit stays set (ix never written). The existing slot semantics model
the register move correctly for free. This is the strongest reuse argument: the subtle part
is already written and tested for 68k.

**Design call: parameterize `preserves.rs` over a `RegisterFile` + a save-op predicate**
rather than fork a `z80_preserves.rs`. The bailout logic is the part that must not drift
(assembly-is-assembly soundness); duplicating it invites exactly the divergence the
campaign's provenance/verify rules exist to prevent. The CFG builder is already shared and
CPU-agnostic; the only CPU-specific inputs are "what is a push/pop/save" and "what registers
exist" — both clean seam parameters. (If the parameterization proves to entangle the 68k
fast path, fall back to a sibling that CALLS the shared CFG — but the default is genericize.)

### 4.3 Shadow-swap subtlety — the proof's forward-compat, not a rung-2 wiring

When `exx` appears (rung 4), the symbolic stack model needs one rule: `exx` swaps the
active/shadow bank, so the entry-value bits for `{bc,de,hl}` swap with their `{bc',de',hl'}`
shadows; a BALANCED `exx … exx` preserves the main bank (bits swap back), a LONE `exx`
leaves the shadow active = the main bank reads as clobbered from the caller's view. `ex
af,af'` is the same for `{a,f}`. This is a clean extension of the entry-value-bit model (add
shadow bits), stated here so §4.2's parameterization reserves room for it. NOT wired in
rung 2 (no demand).

### 4.4 di/ei lattice — not designed (no demand)

Per §1.5, checked and absent from psg/fm. The 3-point MUST lattice (di-held / enabled /
unknown, the `[bus.*]`-style machine-state shape the recon §4.2 sketches) is named for the
rung-4 driver top file. Rung 2 designs nothing here.

### 4.5 Transition for the THREE existing `(cpu: z80)` modules (t27 rule retirement)

The port-loop step-2 t27 rule (`campaign-port-loop.md` §Step 2 item 5) says: OMIT the
register contract on a proc whose CPU has no contract model, because an empty `clobbers()`
falsely reads as "verified: touches nothing." Rung 2 RETIRES that rule for Z80 — but the
retirement is per-proc-that-has-a-contract-to-state, and the three landed modules split
cleanly:

- **`seq_opcode_tab.emp`, `dac_sample_tab.emp`** — PURE DATA (no code procs). The t27
  guidance already lets a genuinely-register-free data proc keep an honest empty
  `clobbers()`. Nothing changes: they stay outside the checker (no `preserves`/`invariant`
  to prove).
- **`z80_init.emp`** — the idle leaf. Its pops are a stack-DRAIN at init
  (`pop ix/iy/de/hl/af/bc` to clear the boot stack), an UNBALANCED pop by design, not a
  save/restore. When the checker engages, z80_init either stays contract-less (a leaf with
  no callers and no preserve obligation — the honest state) OR the checker learns the
  init-drain idiom (an entry-context "the stack is not ours" flag). Since z80_init declares
  no `preserves` and has no `invariant` module attribute, the checker simply finds nothing
  to prove — the transition is a no-op for it. **The retirement bites only when a Z80 proc
  DECLARES a Z80 contract**, which first happens with psg/fm. So: psg/fm procs now WRITE
  `clobbers`/`preserves`/`out` on Z80 registers and carry the inherited `invariant(ix)`; the
  three landed modules are undisturbed.

The step-2 checklist line updates (feed-forward rule): "on a `(cpu: z80)` proc, WRITE the
register contract (clobbers/preserves/out on Z80 regs) once the module carries `(cpu: z80)`
— the t27 omit-rule is retired for Z80 at rung 2; pure-data procs keep the honest empty
`clobbers()`."

---

## 5. (d) The `jr → jp` relaxation ladder (D2.18 / S2-D13(b))

### The reservation

S2-D13(b) (spec line 951): "Z80 `jr → jp` on the same ladder core (when it lands, Z80
positions become provisional for `here()`)." D2.18 (line 170): the `RelaxLadder` core is
CPU-AGNOSTIC — "reach derived from each candidate's fixup KIND, so … a Z80 `jr→jp` ladder
reuses it." `relax.rs:461-509` (`candidate_reaches`) already anticipates it: "a future Z80
`jr → jp` ladder just carries `Z80JrRel8` candidates and this function grows a new arm."

### Direction of relaxation

Grow-only, smallest→largest, exactly like 68k `bra.s → bra.w → jmp`:

- rung 0 (2 B): `jr [cc,] e` — `Z80JrRel8` fixup, ±128, already the symbolic-jr lowering T1
  landed (`lower/code.rs`), range-checked at link (`lib.rs:460-479`).
- rung 1 (3 B): `jp [cc,] nn` — a new `Z80JpAbs16` candidate (16-bit LE absolute; reuse the
  `Value16Le` family T1 already wired for `ld rr,nn`).

`djnz` is short-only (no long form) — never a ladder, always `Z80JrRel8`; an out-of-reach
`djnz` stays a hard `[branch.out-of-reach]` error (a real code-structure problem, matching
asl). `call` is long-only (no short call) — never a ladder. So the ONLY Z80 relax ladder is
`jr↔jp` and `jr cc↔jp cc`.

### Interaction with the existing core

The candidate list is `[Z80JrRel8 (2B), Z80JpAbs16 (3B)]`; `candidate_reaches` grows one arm
returning "does the resolved target sit within ±128 of the jr instruction-end" for the
`Z80JrRel8` rung and "always" for `Z80JpAbs16`. The grow-only rung fixpoint
(`relax.rs:510+`) needs no change — it is fixup-kind-driven by construction. The
construction-contract list at `relax.rs:503` (`PcRel8/PcRelDisp16/Abs16Be/Abs32Be`) gains
`Z80JrRel8`/`Z80JpAbs16`.

### Byte-neutrality — rung 2 stays byte-locked vs asl

Blob-precedes-engine: any site that relaxes DIFFERENTLY than asl chose slides the whole
corpus (recon §1, the t25 STOP rule). asl already picked jr where it reaches and jp where it
doesn't; if our ladder reproduces asl's exact per-site choice, byte movement is ZERO — which
the acceptance gate (§7) proves. So the ladder is LATENT CAPACITY in rung 2, not an
exercised transform: psg/fm are ported with their asl-chosen jr/jp EXPLICIT, and the ladder
must select the identical width at every site. Expected movement: zero; nonzero = STOP.

### The hot-path structural-pin obligation (named, deferred to rung 4)

recon §4.3: on the DAC hot loop, `jp cc` where `jr cc` would reach is LOAD-BEARING (10 vs
12/7 T-states) — the WIDER form is the optimization target, so the ladder must NOT narrow it.
psg/fm are OFF the hot loop, so no hot-path pin is needed at rung 2. But the pin MECHANISM is
named now: a hot-loop `jp` is a STRUCTURAL width-pin (like a 68k stride-locked jump-table
`bra.w`), carrying a site comment, and the T-state feature (rung 4) is what proves the pin is
required. Rung 2 must not auto-narrow, and must not introduce an unsized Z80 branch idiom
that would (§ open question 1).

### Should Z80 adopt an unsized `jbra`-style idiom in step 2?

The 68k step-2 rule sends all control flow new-style (`jbra`/`jbsr`). For Z80 the drafter's
recommendation is **NO unsized branch idiom at rung 2 — keep explicit `jr`/`jp`/`call`/`djnz`
as the Z80 house style** — because (a) on Z80 the branch WIDTH is semantically load-bearing
(T-states), so hiding it behind an auto-selector fights the very property rung 4 must pin;
(b) `jr`/`jp` are already terse (unlike `bra.s`/`bra.w`), so the readability argument for
`jbra` does not transfer; (c) the byte-locked corpus makes an auto-selector pure risk (any
divergence from asl = whole-corpus slide) with zero upside. The `jr→jp` LADDER is still built
and wired (latent, for a future file with a genuine out-of-reach `jr` that asl itself would
relax) — but the SOURCE stays explicit-width, a deliberate Z80 divergence from the 68k
new-style rule, logged as a step-2 noticing-clause entry. **Overseer ruling requested (§Q1).**

---

## 6. (e) The rung-2 test obligation from T1 §0

T1 implemented the `Value::Z80Reg`-in-a-68k-section `[asm.splice-kind]` error but has NO
source producer (only the 68k-reg-in-Z80-section direction is source-tested). §2.3 supplies
the producer: a Z80 proc register param (`Fm_TransposeClamp (ix: *SeqChannel …)`) binds `ix`
to a `Value::Z80Reg`. The rung-2 tranche MUST add the source-level test:

- **T1 §0 obligation**: a Z80 proc binds `ix` (→ `Value::Z80Reg`); splicing `{ix}` into an
  operand slot of an instruction in a `(cpu: m68000)` section → `[asm.splice-kind]`. The
  positive control (t24 rule): the SAME `{ix}` spliced in the Z80 section compiles. This is
  item 4 of the §7 ladder, and it is the first rung-2 item because it is the smallest thing
  that needs the §2.3 typed-param binder.

---

## 7. (f) TDD implementation ladder (acceptance corpus = `sound_psg.asm`)

Every item lands with a demanded-by-a-real-file test; negative checks carry a positive
control (t24 rule); frontend tests compare `.emp` → bytes against the asl golden.

1. **Z80 reglist recognizer (§2).** `expand_reglist_regs(segs, cpu)` parses
   `clobbers(af, b)` / `preserves(c, de, hl, ix)` under a Z80 module to the unit set
   `{a,f,b}` / `{c,d,e,h,l,ix}`. Test: pair-expansion (`de→{d,e}`), half-split
   (`clobbers(b)` leaves `c` unlisted), and the negative control — `clobbers(d0)` under a
   Z80 module is `[contract.unknown-register]` (a 68k name in a Z80 reglist), and
   `clobbers(af)` under a 68k module is the same in reverse.
2. **`out` + flag-result on Z80 regs (§2.2).** `out(hl)` and `out(carry: found)` parse and
   feed the caller-side flag check; a caller that `jr z` on the carry result is credited,
   one that abandons it fires `[call.flag-result-unused]`. Corpus: `PsgVolEnv_Resolve`.
3. **`(ix+d)` + `bit`/`set`/`res` operand wiring (§1.6).** `ld a,(ix+sc_route)` →
   golden bytes; `set SCF_KEYED_B,(ix+sc_flags)` → golden; `bit 4,a` → golden. Negative:
   `(ix+128)` → i8-range error, `(ix+127)` accepted (T1's shared disp8 check).
4. **Typed Z80 proc register params + the T1 §0 splice test (§2.3, §6).** `proc F (a: u8,
   ix: *SeqChannel)` binds `ix→Value::Z80Reg`; `{ix}` splices in the Z80 section
   (positive control) and errors `[asm.splice-kind]` in a 68k section (the obligation).
5. **push/pop preserves proof (§4.2).** (a) a proc that `push hl / call Clobberer / pop hl`
   PROVES `preserves(hl)`; (b) a proc that writes hl and does NOT restore it, declaring
   `preserves(hl)`, fires `[proc.preserves-unverifiable]` (negative); (c) `push ix / pop hl`
   proves `preserves(ix)` AND clobbers hl (the move idiom); (d) `ex (sp),hl` bails the proof
   → a declared preserve over it is `[proc.preserves-unverifiable]` (the rung-3 alias hazard,
   tested now so the bailout is proven). Corpus: `Psg_SetVolume` (push/pop around
   `Snd_ChanClass`), `Psg_EmitDivisor` (push de/pop de).
6. **Module `invariant(ix)` inheritance (§3).** (a) `module … (cpu: z80, invariant:
   preserves(ix))` inherits `preserves(ix)` onto a proc with no explicit contract, PROVEN
   (no ix write); (b) a proc that adds `pop ix` without a matching save fires
   `[proc.preserves-unverifiable]` via the INHERITED invariant (the psg-header-line-60 bug
   class, now a compile error); (c) a proc that re-establishes an invariant it declares it
   breaks is accepted (the grammar path, even if $2A moves to the hardware lint per §3.3).
7. **The t27-retirement transition (§4.5).** `seq_opcode_tab`/`dac_sample_tab` keep the
   honest empty `clobbers()` and the checker finds nothing; z80_init's unbalanced init-drain
   pops do NOT fire an unbalanced-stack error (entry-context or contract-less leaf).
8. **jr→jp ladder latency (§5).** A `(cpu: z80)` section with a symbolic `jr` whose target
   is in ±128 stays 2 bytes (`Z80JrRel8`); a synthetic out-of-reach target relaxes to `jp`
   (3 bytes, `Z80JpAbs16`) — proving the ladder wires — while every psg/fm site selects the
   asl-identical width (byte-neutrality). `djnz` out of range stays a hard error.
9. **Acceptance — port `sound_psg.asm` byte-identical (§10).** psg's `.emp` assembles
   byte-for-byte to the asl-built resident-blob slice under the strict-paired run, with the
   full Z80 contract set (clobbers/preserves/out + inherited `invariant(ix)`) present and
   green. Then `sound_fm.asm` the same way (the invariant-heavy file; §3/§9-A are proven
   against it). Nonzero delta = STOP.

---

## 8. (g) Explicitly OUT of rung 2

- **The interpreter structs + prefix-mirror** (`SeqChannel`/`SfxChannel` single-sourcing,
  the `extends .prefix(57)` spelling, `u16be` packer cells) — rung 3 (recon §4.5). psg/fm
  CONSUME these offsets (`sc_route`, `sc_volume`, `sx_gain`) but do not DEFINE the structs;
  rung 2 reads them as existing const offsets.
- **Shadow-set vocabulary (`exx`/`hl'`) and the di/ei lattice** — rung 4 (§1.5, §4.3, §4.4).
  Represented far enough that the preserve proof stays sound; not wired.
- **T-state / cycle-exact accounting** — BUILD-later, rung 4 (ruling 2). **The seam rung 2
  must preserve**: the contract checker reads each proc's `CodeBuf` / `CodeItem::Instr`
  stream (resolved mnemonics + ops) and MUST NOT consume or rewrite it destructively — the
  same immutable Instr stream is the substrate a future `cycles()` fold reads (T1 §6). Rung 2
  adds a read-only analysis pass; it does not reshape the Instr list. The one place cycles()
  and the relax ladder will MEET is the hot-path `jp`-not-`jr` structural pin (§5) — named,
  not built.
- **The `$2A`-parked / `$2B`-never-written hardware-write disciplines** — a Z80 hardware-port
  lint (`[fm.dac-repark]`, sibling of `[bus.*]`), out of the register-contract core (§3.3,
  §9-B). Named as a fm follow-on, not rung-2 core.
- **Banking-window pointer newtypes** (`ResidentPtr`/`WinPtr`/`BankId`, recon §4.4) — psg/fm
  reach banked tables via T1's `winptr`/`bankid` link-imm machinery; the provenance newtypes
  are a later type-layer pass, not a rung-2 contract concern.

---

## 9. Discrepancies (the tree wins; flagged for the overseer)

- **9-A — `de = $4001` is NOT a module invariant.** Recon §4.2 / §5 frame it as "every proc
  implicitly `preserves(de)` unless it re-establishes." The tree: psg CLOBBERS de and relies
  on the Timer-A tick CALLER to re-establish it (`sound_psg.asm:18-21`); fm CLOBBERS de in
  `Fm_PatchPtr`/`Fm_SetVolume`/`Fm_PatchLoad` (`:113`, `:342`, `:159`) and relies on the
  DAC/idle loop's push/pop bracket around the whole `Sequencer_Frame` (`:13-15`) plus
  absolute-YM-addressing. Neither psg nor fm re-establishes de itself. So de=$4001 is a
  CALLER-BRACKETED, driver-lifetime invariant owned by rung 4, not a rung-2 module invariant.
  The clean rung-2 module invariant is `invariant(ix)` alone (§3.3).
- **9-B — `$2A` parked is a hardware-port discipline, not a register invariant.** Recon §4.2
  lists "reg $2A parked" beside de and ix as "standing register invariants." $2A is a YM
  address-port latch, not a Z80 register — the register-contract model cannot express it. It
  is maintained by re-establishment (`Fm_ReparkDac` tail, `sound_fm.asm:81-84`), which fits
  ruling 3's "unless it re-establishes" phrasing but in the HARDWARE-lint family, not the
  register-preserve family (§3.3, §8).
- **9-C — rung 2's checker demand is narrower than recon §5.** Recon §5 lists rung 2 as
  "push/pop preserves checking, shadow-set vocabulary, di/ei lattice." Grepping the actual
  rung-2 files: NO `di`/`ei`/`exx`/`ex af,af'`/`af'`/`hl'` anywhere (§1.5). The shadow set
  and di/ei lattice are rung-4 (driver top) demands; the `ex (sp),hl` trampoline is rung-3
  (sequencer). Rung 2's ONLY new checker capability is push/pop preserves + `invariant(ix)`
  inheritance. Shadow/di-ei are represented for soundness (§4.3/§4.4) but not wired or tested
  in rung 2.
- **9-D — no discrepancy, a confirmation**: recon §4.5's "`(ix+d) ≤ +127` check on every
  struct field" — every SeqChannel/SfxChannel field psg/fm index is < 128 (structs are 60 /
  68 B), so the guard never fires on the current corpus; it is wired anyway as future-proofing
  (a struct growth would silently corrupt `(ix+d)`), which matches the recon's intent.

---

## 10. Byte-movement statement

**ZERO.** The checker, the `invariant` attribute, the Z80 reglist recognizer, the jr→jp
ladder wiring, and every test are sigil-side. The two ports (psg then fm) are byte-locked
against the asl-built resident blob by the §7 acceptance gate (items 9). Any nonzero movement
at either port is the STOP condition (blob-precedes-engine re-baselines the whole corpus),
never an absorb — the t25 rule the recon §1 states.

---

## 11. Open questions needing an overseer ruling

1. **Z80 branch idiom at step 2 (§5).** Recommend NO unsized `jbra`-style Z80 idiom — keep
   explicit `jr`/`jp`/`call`/`djnz` as Z80 house style (branch width is T-state-load-bearing;
   auto-selection fights the rung-4 hot-path pin and risks whole-corpus slide). The jr→jp
   ladder is still built (latent). This is a deliberate Z80 divergence from the 68k
   "all control flow new-style" step-2 rule — ratify or overrule.
2. **`preserves.rs` genericization vs sibling (§4.2).** Recommend parameterizing the existing
   symbolic-stack proof over a `RegisterFile` + save-op predicate (reuse the tested bailout
   logic; the CFG is already shared) rather than forking `z80_preserves.rs`. Confirm the
   reuse-over-fork call, or direct a sibling if the 68k fast path must stay untouched.
3. **`$2A`-parked / `$2B` hardware-write lint home (§3.3, §9-B).** Confirm these move to a
   Z80 hardware-port lint (`[fm.dac-repark]`, `[bus.*]` family) rather than the register
   `invariant` class — i.e. rung-2's module invariant is `invariant(ix)` only, and the
   port-time $2A discipline is checked (if at all) by a separate fm follow-on lint.
4. **`invariant` attribute grammar (§3.1).** Ratify attaching `invariant: preserves(ix)` as a
   module-header attribute key beside `cpu:` (reusing `ModuleDecl.attrs`), vs a distinct
   top-level `invariant` statement. The attribute form is the minimal-parser-delta choice and
   the T1-named forward-compat slot.
5. **Fall-through routine chains (checker input, noted in §1.6 context).** psg/fm chain
   routines by fall-through (`Psg_ApplyMod`→`Psg_EmitDivisor`→`Psg_EmitDivisorTo`
   `:294-334`; `Fm_NoteOn`→`Fm_NoteOnFreq`→`Fm_NoteOnFreqExact` `:783-826`), each documented
   with its OWN contract but reached without a branch. The checker must attribute the
   successor's effects to the fall-through predecessor (a fall-through edge kind), OR the
   ports fold each chain into one proc with internal labels. Which shape does the port take —
   separate contracted procs + a fall-through edge in the closure/preserve proof, or merged
   procs? (Byte-critical: a jump cannot be inserted between them.) Flagged for the tranche
   brief; leaning separate-procs-with-fall-through-edge to keep the per-routine contracts the
   headers already document.

---

Draft path: `docs/superpowers/notes/2026-07-29-z80-rung2-contracts-DRAFT.md`
(uncommitted; overseer reviews before it lands, per the T1 flow).

---

## 12. OVERSEER RULINGS (Fable, 2026-07-29 — reviewed against the tree before ratification)

Countersign notes: discrepancies 9-A/9-B/9-C were re-verified with the overseer's own reads
(9-A: `sound_fm.asm:10-19` preserves de=$4001 BY CONSTRUCTION — never loads a port address
into de — while `sound_psg.asm:17-21` CLOBBERS de with the Timer-A caller re-establishing;
both facts confirm de is NOT a rung-2 module invariant. 9-C: the single `di` grep hit in fm
is `sound_fm.asm:713`, a comment. 9-B is self-evident from the fm header's re-park
discipline — $2A is a YM address latch, not a Z80 register). The recon note's §3/§5 claims
are superseded by this draft's §1 demand tables where they disagree.

1. **RATIFIED — no unsized Z80 branch idiom at step 2.** Explicit `jr`/`jp`/`call`/`djnz`
   is Z80 house style; branch width is T-state-load-bearing and auto-selection fights the
   rung-4 hot-path pin. The jr→jp ladder is still built (latent capacity, byte-locked).
   This is a RECORDED DELIBERATE DIVERGENCE from the 68k all-control-flow-new-style rule;
   it enters the port-loop step-2 checklist when rung 2 lands. Volence override slot open.
2. **RULED — GENERICIZE `preserves.rs`, do not fork**, under a hard 68k-frozen bar: the
   `RegisterFile` + save-op parameterization must leave 68k behavior bit-identical, proven
   by the full strict suite with ZERO 68k-path test churn beyond mechanical type plumbing,
   and the landing commit carries an explicit no-behavior-change statement (the t25
   parser-guard precedent). If honoring the bar forces contortions, STOP and switch to the
   sibling — report, don't absorb.
3. **CONFIRMED — `invariant(ix)` is rung 2's SOLE module invariant.** The $2A/$2B port
   discipline moves to a named hardware-port-lint SEAM (`[fm.dac-repark]` / `[bus.*]`
   family) recorded as a ledger row, DESIGN-ONLY now, wired only when the fm rung demands
   it. Nothing port-lint-shaped is built at rung 2.
4. **RATIFIED — the module-header attribute form** (`invariant:` beside `cpu:`, reusing
   `ModuleDecl.attrs`). This is exactly the forward-compat slot the T1 design named for
   ruling 3; minimal parser delta; a top-level statement form can be revisited if the
   attribute list ever crowds.
5. **RULED — separate contracted procs + fall-through.** The per-routine contracts the
   headers already document are the point of the port; merging loses them. MECHANISM
   CONSTRAINT: the house already has `falls_into` (t22, `S4LZ_DecompressDict falls_into
   S4LZ_Decompress`) — the implementation EXTENDS `falls_into` to Z80 procs and teaches the
   closure/preserves proof its edge, rather than minting a parallel edge kind. If
   `falls_into`'s current semantics genuinely cannot carry the Z80 case, that is a finding
   to report with evidence, not a license to fork the concept.

DISPOSITION: draft RATIFIED with the five rulings; renamed to
`2026-07-29-z80-rung2-contracts.md` and committed on sigil master (doc-only, queue-safe).
Implementation dispatches on its own branch AFTER t28's countersign cadence allows —
the TDD ladder §here items run with sound_psg as acceptance corpus; the T1 §0 splice-test
obligation rides ladder item 4 as drafted.

---

## 13. IMPLEMENTATION ADDENDUM (branch `z80-rung2-contracts`, this pass)

### 13.1 §9 discrepancies caught during implementation (tree wins)

- **9-E — proc bodies do NOT allow operand splices.** §2.3/§6 sketch `{ix}` spliced
  directly in a proc body. The parser gates `{r}` operand splices to `comptime fn`
  TEMPLATE bodies (`parser.rs` `splices_allowed`); a proc body is not a template. The
  Z80 register-named param DOES bind to a `Value::Z80Reg` (the producer, landed item 4),
  but reaches a `{r}` splice by flowing through a comptime-fn template the proc
  instantiates — the SAME vehicle T1's own 68k-direction splice test uses. The section
  CPU the template instantiates under decides validity. (No behavior gap — the obligation
  is discharged; only the SPELLING differs from §2.3's sketch.)
- **9-F — `check_flag_unused`/`consumes_carry`/`writes_carry` are NOT CPU-agnostic.**
  §2.2 states the caller-side flag check "is CPU-agnostic over `CodeItem::Instr` and needs
  only the Z80 condition-mnemonic recognition." In the tree they are hardcoded 68k
  mnemonic ALLOWLISTS (`bcs`/`bcc`/`bhi`/`bls` consumers; the 68k CC-writer set). Z80
  flag-result support (item 2) requires teaching them the Z80 condition mnemonics
  (`jr z`/`jr c` consume; the Z80 CC-writers redefine) — an ADDITIVE touch of the 68k
  flag path (new Z80 arms, 68k allowlists byte-unchanged), per the overseer's item-2 bar.

### 13.2 Overseer countersign (2026-07-29) — item 5 = SIBLING, ratified

Ruling 2 amended on its own terms: the shared soundness bailouts the original rationale
protected are predominantly 68k-specific (movem masks, linear-delta arithmetic,
sp-displacement hazards); the genuinely shared part — `flag_check::Cfg` — is already
shared. So a `z80_preserves` SIBLING duplicates nothing that matters. Constraints: the
sibling CALLS the shared `Cfg`; mirrors the join / call-clobbers-all-nets-zero
conventions (cite the `preserves.rs` lines mirrored); the `push R / pop R'` move idiom is
in scope; `ex (sp),hl` is a REPRESENTED loud bailout (rung-3 wires it); `preserves.rs`
stays byte-untouched; TDD with t24 positive controls. Continuation authorized for the full
checker cluster: item 5 sibling → item 6 inheritance proof → item 2 flag results (additive
68k bar) → `Z80Cc` eval producer (only if demanded) → checker routing (`analyze_z80_module`
replaces the skip; the three existing Z80 modules compile unchanged = an executable item-7
test). End state: rung-2 sigil-side COMPLETE except item 9 (the psg/fm ports).

### 13.3 HANDOFF — what remains for a fresh implementer (branch tip after items 1-8)

LANDED this pass (frozen-68k bar held, suite 2685→2713): item 1 (reglist recognizer,
`regfile.rs`), item 3 (bit/set/res + Z80Bit), item 4 (Z80 proc params + T1 §0 splice),
item 5 (`z80_preserves` SIBLING proof), item 6 (invariant grammar + inheritance), item 7
(executable vacuous-pass test), item 8 (jr→jp latent ladder). preserves.rs/flag_check.rs
byte-untouched.

PLACEMENT NOTE for the checker (countersign step 5): the single-proc preserves +
invariant proof is wired at the **per-proc lowering** seam (`lower_proc` →
`check_z80_preserves`), where 68k's `check_preserves` also runs and where firings reach
`lower_module` diagnostics directly — NOT (yet) inside `analyze_corpus`'s `module_is_z80`
skip. This is sound: the sibling proof conservatively treats every `call` as
clobber-all, so it needs no transitive closure. The corpus skip stays correct for the 68k
closure. The executable item-7 vacuous-pass test lives at this seam.

REMAINING — **item 2 (flag results on Z80 regs)** + its dependencies, a coherent chunk
deliberately NOT built (deep-in-budget stop; the invasive frozen-`flag_check` touch wants
a fresh, unhurried pass):

1. **`Z80Cc` eval PRODUCER** (currently the bounded-scope negative — `ret nz` is
   `[lower.z80-unsupported]`). A condition code in control-flow FIRST-operand position
   (`jr z`/`jr c`/`ret c`/`call nz`) must map to `CodeOperand::Z80Cc`. HAZARD: `c` is
   ambiguous — the C register (`Z80Reg8::C`, produced today) vs the carry cc. Resolve like
   the AS front-end's `control_flow && i==0` rule (`eval.rs:3601`): under jr/jp/call/ret,
   the first operand is a cc. The lowering `CodeOperand::Z80Cc → Z80Operand::Cc` arm
   already exists (T1); only the eval producer is missing. Cleanest seam: `map_z80_operands`
   (has the mnemonic) reinterprets a leading register/Sym as a cc for control-flow forms,
   mirroring the item-3 bit-number handling.
2. **`flag_check` Z80 arms — ADDITIVE, hard 68k bar** (countersign item-2 constraint):
   `consumes_carry`/`writes_carry` (`flag_check.rs:77`,`:100`) are 68k mnemonic ALLOWLISTS
   (discrepancy §9-F). Add Z80 arms (`jr c`/`jr nc`/`ret c` consume carry; the Z80
   CC-writers redefine) as NEW branches, 68k allowlists byte-unchanged; the full suite is
   the proof. If additivity fails, STOP with evidence.
3. **Corpus routing for the CROSS-PROC flag analysis** — item 2's caller-must-consume check
   (`PsgVolEnv_Resolve` declares `out(carry: found)`; a caller `jr z` on it is credited,
   one that abandons it fires `[call.flag-result-unused]`) is inherently cross-proc, so Z80
   procs' flag callees/consumers must reach the whole-corpus flag pass. Either un-skip
   `(cpu: z80)` for a Z80-aware flag sub-pass in `analyze_corpus_with`, or a sibling
   `analyze_z80_module` (design §4.1). `out(hl)`/`out(carry:)` PARSING already works
   CPU-agnostically (`out_list`); the caller-side check needs the Z80Cc recognition from (1).
4. **`Z80Cc` on the `jr` ladder** — once (1) lands, a symbolic `jr cc,L` can join the item-8
   ladder (conditional `jr cc` → `jp cc` rung); today only unconditional `jr` ladders.

Corpus demand for item 2 is real (design §1 table: `sound_psg.asm:120` `out(carry)`), so it
is in rung-2 scope — but it is the ONE remaining ladder item, and the psg/fm ports (item 9)
are the acceptance gate for the whole rung.

### 13.4 IMPLEMENTATION — item 2 LANDED (finisher pass, four sub-part commits)

All four §13.3 sub-parts landed; suite **2713 → 2729** (0 failed, 1 ignored), frozen-68k
bar held throughout. Commits (one per sub-part):

- **2.1 `e633772` — `Z80Cc` eval producer.** `eval/asm.rs` `lower_instr_to_item`: under a
  Z80 `jr`/`jp`/`call`/`ret`, a bare condition word in the FIRST operand position becomes a
  `CodeOperand::Z80Cc` (helpers `is_z80_control_flow` / `z80_cc_operand`). Lowering
  (`lower/code.rs`) grows a conditional `jr cc` rung, `ret cc` via the comptime encode path,
  and `jp/call cc, Label` via the symbolic-abs16 path. Flips the former bounded-scope
  negative (`ret nz`) to a positive; adds the `ld a, c` positive control (t24).
- **2.2 `d4d340b` — additive Z80 arms in `flag_check`** (§9-F). `consumes_carry` /
  `writes_carry` gain a `cpu` param + a leading `if cpu == Cpu::Z80 { return z80_… }` guard;
  new `z80_reads_carry` / `z80_writes_carry` helpers; new `Cfg::z80_edges` method;
  `check_flag_unused` / `abandons_flag` gain `cpu`; `is_call_site` recognizes the Z80 `call`.
- **2.3 `00f230f` — corpus routing** (§4.1). A SEPARATE `Cpu::Z80` flag pass
  (`collect_z80_flag_procs`) in `analyze_corpus_with` routes Z80 procs' flag callees + bodies
  into `check_flag_unused`; the 68k closure stays Z80-free.
- **2.4 `69ff308` — conditional `jr cc` ladder** (§5). `lower_jr_jp_cc_candidates` (encoder-
  sourced opcodes) upgrades the sub-part-1 plain form to the grow-only `jr cc → jp cc` ladder.

**13.4-A — the `c`/carry ambiguity resolved (§13.3 item 1).** As the handoff specified:
`control_flow && i == 0 ⇒ cc`. Under `jr`/`jp`/`call`/`ret`, position 0 is a condition; every
other position and mnemonic reads `c` as the C register. VERIFIED against the encoder
(`z80.rs`): `ret cc` (`C0|cc<<3`), `jp cc, nn` (`C2|cc<<3`), `call cc, nn` (`C4|cc<<3`),
`jr cc, e` (`20|cc<<3`, `cc < 4`) all encode — the rule is sound, NO STOP. The positive
control `ld a, c → 79` (register, unchanged) fences it.

**13.4-B — DISCREPANCY (tree wins): the producer seam is EVAL, not `map_z80_operands`.**
§13.3 item 1 named `map_z80_operands` (a LOWERING function `CodeOperand → Z80Operand`) as the
seam. The tree requires the EVAL layer (`lower_instr_to_item`, `CodeOperand`-producing)
instead, because item 2's flag analysis reads the `CodeItem::Instr` `ops` stream — the cc must
be a `Z80Cc` IN THE CODEITEM (so `consumes_carry`/`z80_edges` see it), not merely
reinterpreted at encoding. Reinterpreting only at `map_z80_operands` would leave the CodeItem
carrying `Z80Reg8(C)` / a mangled `Sym("nz")` (post-`resolve_ref`), which the flag walk cannot
read. Producing `Z80Cc` at eval feeds BOTH consumers cleanly (the T1 `Z80Cc → Z80Operand::Cc`
lowering arm already exists). No behavior gap — the obligation is discharged; only the seam
differs from the handoff's sketch.

**13.4-C — additivity proof for sub-part 2 (the item-2 hard bar).** The 68k allowlists are
byte-unchanged: `consumes_carry` / `writes_carry` reach their original `matches!(…)` /
`CALL_MNEMONICS` allowlists via the SAME path for 68k (the Z80 branch is an early
`if cpu == Cpu::Z80 { return }` guard placed BEFORE them), and `Cfg::edges` is untouched (Z80
uses a NEW `z80_edges` method). Proof: the full suite — every prior `flag_check` + corpus
test — stays green (2724/0 at the sub-part-2 commit), and the 3 landed Z80 modules (which
declare no `out(carry:)`) produce zero Z80 flag firings on the real corpus (firing-neutral).
68k callers pass `Cpu::M68000` (mechanical plumbing, the anticipated churn). Additivity did
NOT fail — no STOP.

**13.4-D — corpus routing needs MODULE-level `(cpu: z80)`.** `module_is_z80` reads
`file.module.attrs`, so the §4.1 flag pass routes only when the CPU is on the MODULE header
(`module m (cpu: z80)`), as the real corpus declares it (`module engine.z80_init (cpu: z80)`).
A section-only `(cpu: z80)` would leave the 68k PASS-2 to (mis)handle the procs. Not a change —
a constraint the synthetic tests must honor (and do).

**13.4-E — SOUNDNESS FIX (overseer-authorized this pass): `z80_preserves` conditional-branch
edges.** `z80_preserves`' private `z80_edges` (`z80_preserves.rs`) now treats a CONDITIONAL
form (a leading `Z80Cc`) as a genuine two-way split: `jr cc`/`jp cc` contribute BOTH the taken
edge and the fall-through; `ret cc` contributes the return (abandon) AND the fall-through.
Unconditional `jr`/`jp`/`ret` stay single-edge. This mirrors the flag check's `Cfg::z80_edges`
(`flag_check.rs`) — one conditional-split edge model across both Z80 dataflows.

The bug it closes: the prior `z80_edges` treated EVERY `jr`/`jp` as unconditional (one
`Follow` to the target), DROPPING a `jr cc` fall-through — so a register clobbered only on
that path was missed and a `preserves` over it was WRONGLY verified (a `preserves` that lies
is worse than no proof). The gap pre-dated item 2 but item-1 makes conditional `jr cc` a
first-class lowering form, so it became exercisable. PERMANENT reproducer +
positive control now in the proof's test set:
`z80_conditional_jr_fallthrough_clobber_fires` (a clobber on the `jr z` fall-through fires
`[proc.preserves-unverifiable]`) and the t24 `z80_unconditional_jr_keeps_single_edge` (an
unconditional `jr .skip` stays single-edge, so the dead `ld a, 5` it jumps over does NOT
false-fire — the guard against a phantom fall-through leaking onto unconditional branches).
`preserves.rs` (the 68k proof) stays byte-untouched.

**Rung-2 sigil-side is now COMPLETE except item 9 (the psg/fm byte-locked ports).**

---

## 13.5 THE psg ACCEPTANCE CORPUS CLOSED THREE UNDER-WIRED GAPS (t32 finisher pass, overseer-dispatched)

Item 9 began: the t32 porter built the `sound_psg` windowed oracle (byte-identical
both shapes) but found the FULL faithful contract set could not go live — the
checkpoint-(a) findings. Applying it fired **63 diagnostics** (38 `clobber-invalid`
+ 5 `out-invalid` + ~20 `preserves-unverifiable`), so psg LANDED with `invariant(ix)`
+ calling-proc `preserves` + `clobbers`/`out(<reg>)` as PROSE. This pass wired the
three gaps the items-1-8 implementation under-built; the full set now VERIFIES
machine-checked, **63 → 0**, `invariant(ix)` LIVE, byte-identical (`sound_psg_port`
5/5). Frozen-68k held throughout (`preserves.rs` byte-untouched; suite 2765 → 2777).

### 13.5-A — the items-1-8 wiring gap (`clobbers`/`out` never reached the recognizer)

§2/§2.2 DESIGNED `clobbers`/`preserves`/`out` to ride the CPU-aware recognizer, but
items 1-8 wired only `preserves` (via `check_z80_preserves` → `expand_reglist`),
`invariant`, and `out(carry:)`. `check_clobbers`/`check_out` (`lower/proc.rs`) still
validated their reglists against the **68k** universe (`reglist_expand`'s
`preserves_reg_bit`), so every `clobbers(af)` / `out(a)` on a Z80 proc fired
`[proc.clobber-invalid]` / `[proc.out-invalid]`. FIX: both gained a `cpu` param; a
Z80 proc routes its reglist through `expand_reglist(RegFile::Z80)`. The `out`
partition also needed the Z80 out∩clobbers / out∩preserves overlap checks Z80-
expanded, and the out-UNWRITTEN check SKIPPED on Z80 (its write detector is the 68k
heuristic — a Z80 `out` is unverifiable-written, honest like `preserves`; an empty
68k `written` set would have false-fired unwritten on every Z80 out — a latent bug
the prose contracts never exercised). This is why the psg `af` header SPLITS in the
`.emp`: `out(a) clobbers(f)` (the result vs the flag scratch), disjoint by
construction.

### 13.5-B — the §3.2 "trivially satisfied" over-claim (contradicted by its own corpus)

§3.2 asserted `invariant(ix)` is "trivially satisfied (no instruction writes ix)"
and "the day a future edit adds `pop ix` … fires." Both halves are true for a
NO-CALL proc — but the sibling proof (§13.3: "the sibling proof conservatively
treats every `call` as clobber-all, so it needs no transitive closure") makes a
`call` write EVERY register. psg is dense with calls, so `invariant(ix)` +
every calling proc's `preserves` fired on the 5 calling procs (Psg_ChBase /
Psg_NoteOff / Psg_ApplyMod / Psg_EmitDivisor / Psg_SetVolume) EVEN THOUGH ix is only
ever read as `(ix+d)`. The §3.2 claim held only because **the item-5/6 tests
exercised NO call case** — every `preserves`/`invariant` fixture was a leaf. The
corpus contradicted the design. This is the IDENTICAL wall t30 hit for 68k
(`verify_preserved` clobber-all through a preserving call before `rts`), which t30
fixed with the callee-preserves oracle — but the Z80 sibling was built call-clobber-
all and never got the equivalent. FIX (gap 2): a `CalleePreserves` map (visible proc
/ `extern proc` → preserved units) — the Z80 analog of `preserves.rs`'s
`CallPolicy::Oracle` (`:77`) / `call_preserves` (`:83`) / `callee_clobbers`' `None
=> true` (`:632`). The `transfer` call arm clobbers only units the callee does NOT
preserve; unknown/indirect → conservative clobber-all. A local proc's map entry is
its declared `preserves` UNIONED with the module invariant (each local proc is
itself checked, so the credit is sound; a caller's own write always clears its bit
regardless of callee credit); an `extern`'s is its declared `preserves` (trusted, the
extern convention). DIVERGENCE from the 68k closure (recorded honestly): the per-proc
map trusts DECLARATIONS directly rather than a verified `effective` fixpoint — the
per-proc seam has no closure — so a declared-but-false `preserves` on a callee would
not itself fold to conservative; it is caught instead by the callee's OWN check
firing (the build stops at the root). Sound because own-writes are always caught and
psg is acyclic; a genuinely cyclic mutual-preserve would be trusted (the 68k closure
would fold it conservative). Named for a future rung if a Z80 cycle appears.

### 13.5-C — the vacuous tail-jp pass (a soundness hole; the never-rides-flagged-open rule)

A proc ending in a tail `jp`/`jr` with NO local `ret` reached only `Edge::Defer` (the
external-tail edge, ignored mirroring `preserves.rs:294`), so `saw_return` stayed
false and the proof returned `Verified` VACUOUSLY (`!saw_return`) — silently
verifying a `preserves` the body breaks. psg's `Psg_NoteOn` / `Psg_EmitNoiseClock` /
`Psg_Noise` all tail-jump (`jp Psg_SetVolume`, `jr Psg_EmitDivisorTo`) and passed
`preserves(ix)` on air. The reproducer, now a permanent test
(`z80_tail_jp_clobber_fires`): `push bc / pop ix / jp External` — clobbers ix then
tail-jumps, MUST fire, but passed vacuously. FIX (gap 3): `Edge::Defer` is now an
EXIT that CHECKPOINTS — the proc preserves rN across a tail transfer iff rN holds
its entry value AT the jp AND the tail-callee itself preserves rN (unknown →
conservative, via the same oracle). A tail-jp to a same-module proc (`jp
Psg_SetVolume`) is credited from the map; an external unknown fires. `saw_return` is
now set at a Defer exit, so the vacuous branch only survives a genuinely no-exit body
(an infinite loop).

### 13.5-D — the local END-label mis-classification (a gap-3 self-catch on the corpus)

Making `Edge::Defer` a checkpoint surfaced a latent ambiguity in `Cfg::label_index`:
it returns `None` BOTH for a truly external symbol AND for a LOCAL label at the proc's
very end (no following instruction). `Psg_ApplyMod`'s `jr z, .div_ok` — where
`.div_ok:` closes the proc before it falls into `Psg_EmitDivisor` — was thus read as
an external tail transfer, and the conservative unknown-callee fired
`[proc.preserves-unverifiable]` on the inherited `invariant(ix)`. FIX:
`flag_check::Cfg` gains an additive `is_local_label` (a `CodeItem::Label` defined
among the items, end-label included); z80_preserves' `branch_edge` routes a local
end-label jump to `Edge::Abandon` (an in-proc fall-off), reserving `Edge::Defer` for
a symbol that is neither a local label nor a local end-label (a genuine tail call) or
a computed `jp (hl)`. This is the whole reason gap 3 must consult label DEFINITION,
not just `label_index` — the corpus caught the shortcut.

### 13.5-E — the honest contracts the checker positively caught (the port's payoff)

With the machine checker live, several 15-year-old `.asm` header over-claims became
honest `.emp` contracts the proof VERIFIES (the "a false clobber comment caused a
prior bug" class the psg header warns of, now a compile-time positive):
`Psg_EnvCursorReset` clobbers NOTHING (the two `(ix+d)` immediate stores touch no
register — the header's `Clobbers af` over-claims; the `.emp` `preserves(af, …)` and
the proof confirms it); `Psg_EmitDivisor` preserves `b`, `de` (de is push/pop-
bracketed across `Psg_ChBase`, b untouched — the `af, bc, de` header over-claims);
`Psg_EmitDivisorTo` / `Psg/FmVolEnv_Resolve` preserve `c` (only the djnz counter `b`
dies — the `bc` header over-claims). The `out`/`clobbers` split of every `af` result
register is the §1.1 register-half design earning its keep.

### 13.5-F — residue the psg corpus still cannot express (honest)

- **No `di`/`ei`/`exx`/`ex (sp),hl` in psg** (§1.5 confirmed on the tree) — the
  shadow-set, di/ei-lattice, and trampoline-alias bailout stay REPRESENTED-not-wired
  (§4.3/§4.4); psg exercises none, so rung 2 leaves them for rung 3/4 as designed.
- **The callee oracle is declaration-trust, not a verified closure** (13.5-B) — a
  cyclic Z80 mutual-preserve would be trusted where the 68k closure folds it
  conservative; psg is acyclic so it never bites, and it is named for the rung that
  first ships a Z80 call cycle.
- **`out(carry:)` cross-proc must-use is untested WITHIN psg** — `Psg/FmVolEnv_Resolve`
  declare `out(hl, carry: found)` but their consumers live in other files (the
  sequencer), so the windowed port has no in-file caller to exercise the
  `[call.flag-result-unused]` check; the whole-ROM seam sub-tranche is where that
  lands.
- **`de = $4001` / `$2A`-parked stay out of the register-invariant model** (§9-A/§9-B,
  ruling 3) — psg CLOBBERS de and relies on the Timer-A caller; the `.emp` honestly
  `clobbers(de)` where it touches it and carries no de invariant, as ruled.

Credit: overseer (Fable) dispatched + drove this finisher pass; the porter's
checkpoint-(a) table (the 63-firing arc, the Psg_EnvCursorReset correction) is the
input this discharges.
