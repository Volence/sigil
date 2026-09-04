# An unknown head under `CPU Z80` refuses; and the gap that refusal exposes

2026-09-03. Branch `z80-unknown-head-loud`, off master `6f0762a3`.

## What the Z80 path did

Under `CPU Z80` an indented head that matched no mnemonic, no directive and no
macro was **bound as a label**: no diagnostic, no bytes, exit 0. An unimplemented
Z80 instruction therefore emitted nothing and the blob came out short with
nothing to read. The same head on the 68000 path was already loud.

The site is **not** the mnemonic-dispatch arm. `dispatch`'s chain ends in
`_ => self.err(…)`, so anything reaching it is loud. The silent bind is upstream,
in the bare-head branch of `exec_line` (`eval.rs`, the
`!is_op_keyword && !is_mnemonic && !macros && !attribute_macro` block): AS's
column rule — column 0 is a label, indented is an instruction — was gated on
`cpu == Cpu::M68000 || fold_kw(head) == "eval"`, so under Z80 nothing routed the
head to `dispatch` at all.

The rule is not a 68000 fallback for the absent m68k mnemonic table. It is what
decides whether an unrecognized head is a *definition* or a *diagnostic*, and
that question is cpu-independent. The fix removes the cpu condition.

## What `asl` actually does — four shapes

S1's own `asl` (upstream AS, md5 `61e672562465725a8c102288a7da9098`), `-U` on
every invocation, under `CPU Z80`:

| source | `asl` | sigil before | sigil after |
|---|---|---|---|
| indented `zqp_bogus` | `error #1200: unknown instruction  ZQP_BOGUS` | silent, exit 0 | names the head |
| column-0 `zqp_bogus` | exit 0 — it is a label | label | label (unchanged) |
| indented `zqp_bogus a,b` | `error #1200`, naming the HEAD | named the first **operand** | names the head |
| indented `ldi` | exit 0 — a real Z80 instruction | silent, exit 0 | names the head |

Row 4 is the class. `ldi` is a Z80 instruction sigil does not encode.

## The defect measured in bytes, not in diagnostics

A diagnostic count cannot see this class by construction — the whole defect is
that nothing was counted. `nop / ldi / nop` at `org 0`:

```
asl            00 ED A0 00     4 bytes
sigil before   00 00           2 bytes, exit 0, no diagnostic
sigil after    error: unknown directive or mnemonic `ldi`, exit 1
```

The blob was short by exactly the two bytes of `ldi`, silently.

## The second effect, which bytes alone would not have found

The phantom label **opened a local-label scope**. Every `.local` defined after
it was qualified under the phantom instead of the real enclosing label, so
references from before it went unresolved. Isolated:

```
	cpu z80
Outer:
	ld	a,(.loc)
	zqp_bogus
.loc:
	nop
```

before → `error: unresolved symbol `.loc` in operand` (and no mention of the
head at all); after → `error: unknown directive or mnemonic `zqp_bogus`` and
`.loc` resolves. This is `s2disasm`'s `.is_psg` and `.voiceptr`: three
unresolved-symbol diagnostics that were collateral damage of the 17 eaten `ldi`
heads above them.

## The gap the refusal exposes

Corpus runs, sigil built from this branch, `sigil <root>` from the corpus root,
no flags. Sets compared **both directions**, not totals.

**Which binary produced these.** `/home/volence/sonic_hacks/.z80eval-target/release/sigil`,
whose `--version` reports revision and closure-revision
`e20206f8e7b730a33c5f37f8a07ce046598017f9`, branch `z80-unknown-head-loud`, tree
clean, source `.z80-gap-wt`. It is byte-identical (`cmp`, distinct inodes) to the
binary the landing run used, so one program produced both the suite result and
these counts. The freshness witness is `closure-revision`, not `revision`: branch
HEAD is `5aee292f`, which touches only `docs/` and is outside the compiled
closure, and `git log -1 --format=%H HEAD -- <closure-paths>` returns `e20206f8`
to match. Cite that, not the binary's md5 — an md5 abbreviated to eight hex
characters is indistinguishable from a short SHA and reads as a followable
citation when it is not one.

| corpus | before | after | net |
|---|---|---|---|
| `s1disasm` `f6ece657` (`sonic.asm`) | 368 | 368 | 0 |
| `s2disasm` `e45ebf3` (`s2.asm`) | 8,918 | 8,932 | +14 |

**+18 new, −4 gone.** Every one accounted for:

| Δ | rows | what |
|---|---|---|
| +17 | `unknown directive or mnemonic \`ldi\`` at `s2.sounddriver.asm` 1873-1876, 1986-1989, 2275, 2276, 2292-2294, 2310, 3486-3488 | the silent gap, now visible |
| +1 / −1 | `listing` replaces `purecode` at `s1 sound/z80.asm(11)` and `s2.sounddriver.asm(251)` | same site, same count; the message now names the HEAD instead of the operand |
| −3 | `unresolved symbol \`.voiceptr\`/\`.is_psg\`` at `s2.sounddriver.asm` 2196, 2206, 2220 | the scope corruption above, repaired |

S1's total is unchanged **and its content is not** — the `purecode` → `listing`
swap is why the sets were diffed rather than the counts.

### The gap as a list

Z80 mnemonics sigil does not implement, whole ISA (**22**):

```
cpd cpdr cpi cpir in ind indr ini inir ldd lddr ldi otdr otir out outd outi
reti retn rld rrd sll
```

Of those, **exactly one is used by either corpus: `ldi`**, 17 sites, all in
`s2.sounddriver.asm`. S1 uses none of them: every mnemonic in `sound/z80.asm`
and in `sonic.asm`'s second Z80 region (323-353) is already implemented.

Seven implemented mnemonics are exercised by neither corpus — `daa halt rl rlc
rrc scf sra` — so corpus coverage is not a proof of the encoder.

## The byte sweep

`scripts/z80_byte_sweep.sh` assembles every self-contained Z80 instruction line
the two corpora use with **both** `asl` and sigil and compares emitted bytes.
Region bounds are found from the `CPU Z80`/`CPU Z80UNDOC` switch to its matching
`restore`/`dephase`, so a corpus edit cannot silently narrow it. `asl` and
`p2bin` come from S1's own `build_tools/Linux-x86_64/`; the md5 is printed in
the header of every run.

999 distinct corpus lines → 255 self-contained. Result on this branch:

```
identical    247
DIFFERS        0
SIGIL-ERR      6     ldi · sbc hl,bc · ld a,ixl · ld e,ixl · ld a,iyl · add a,ixl
skipped        2     jr + · jr z,+   (nameless local labels, no standalone target)
```

**Zero DIFFERS**: where sigil encodes a Z80 instruction at all it agrees with
`asl` byte for byte. The six failures are absent mnemonics and absent operand
forms, and five of the six were already loud before this parcel — the sweep
found them independently of the diagnostic run.

### Proof of sensitivity

A sweep that has never failed is not evidence. `(Mnemonic::Exx, []) =>
Ok(vec![0xD9])` in `crates/sigil-isa/src/z80.rs` was changed to `0xDA`, the
mutation read back from disk with `git diff`, and the mutant built into a
**separate** target dir so the good binary was never overwritten:

```
DIFFERS  exx  | asl=D9 | sigil=DA
identical    246   (was 247)
```

Source restored from the committed baseline, the mutant target dir destroyed,
and the sweep re-run: 247 identical, 0 DIFFERS.

## The gate

Three tests in `crates/sigil-frontend-as/src/eval.rs`, expectations derived from
the `asl` probes above, each shown red under the mutation it exists to catch:

| test | mutation | result |
|---|---|---|
| `unrecognized_indented_head_under_z80_is_loud_not_a_label` | restore the `cpu == M68000` gate | **FAILED** — ``\`zqp_bogus\` assembled silently; it must diagnose`` |
| `an_unrecognized_z80_head_does_not_open_a_local_label_scope` | same | **FAILED** |
| `unrecognized_column_zero_head_under_z80_is_still_a_label` | drop the column test (refuse every unknown head) | **FAILED** (with 3 other pre-existing tests) |

Both mutations were applied by script, shown applied via `git diff` read back
from disk, and reverted with `git checkout` from commit `e20206f8`; the release
binary was rebuilt after each restore.

The third test is the guard against over-correction: it is green under the first
mutation and red under the second, which is why both mutations were run.

**Where the rule is not exercised.** Aeon cannot exhibit this class: its Z80
lives in `.emp` `cpu: z80` sections, lowered by the `.emp` front end, and its
residual `.asm` root is 68000-only — there is no `CPU Z80` reaching the AS
front-end path anywhere in the aeon tree. The gate is exercised by the two
disassembly corpora (17 live sites), not by aeon.

## Fixed versus booked

**Fixed.** The silent bind, and with it the local-label scope corruption.

**Booked, with measured size.**

- **Z80 instruction coverage — a separate row.** 22 mnemonics missing ISA-wide;
  one of them (`ldi`, 17 sites) blocks `s2disasm` today. Also absent, and
  already loud before this parcel: the `sbc hl,bc` 16-bit form (7 `sbc` lines in
  the S2 Z80 region) and the undocumented half-index registers `ixl`/`iyl`
  (4 lines, reported as `unresolved symbol`). Cherry-picking `ldi` was rejected:
  the gap is now *visible*, which is what this row was for, and the 22 are one
  coherent piece of work rather than the one the corpus happens to name.
- **`listing`.** An AS directive sigil does not implement, 2 sites, both
  `listing purecode`. `asl` emits **zero bytes** for it, so it is an emission
  no-op — small, but it is a directive gap, not a Z80 one.

## Whether the end-to-end byte oracle was reachable

**No, and it is worth saying why.** `s1disasm` builds bit-perfect against the
retail REV01 cartridge, which is the strongest oracle in the project — but sigil
does not reach link on either corpus, so no full-blob comparison is available.
S1's `sound/z80.asm` additionally needs `function`-call operands (4 `trailing
tokens in operand`) and `listing`; S2's driver needs `ldi`. The per-line sweep
above is what is reachable today, and it is a byte comparison, not a count.

## What stays open

- The 22 missing Z80 mnemonics, `sbc hl,bc`, and `ixl`/`iyl` (booked above).
- `listing` (booked above).
- `jr +` / `jr z,+` — nameless local labels, the known S2 class; they are the
  sweep's 2 skips.
- Everything the S1 baseline note already books, unchanged by this parcel.

## How verified

**Full suite.** `scripts/landing-run.sh --baseline 4348 --aeon ~/sonic_hacks/.aeon-eval-ref`,
run alone. Log kept at `/home/volence/sonic_hacks/.z80eval-land/landing.log`, stamped
`pwd /home/volence/sonic_hacks/.z80-gap-wt`, `sigil HEAD e20206f8`, branch
`z80-unknown-head-loud (clean)`, `aeon HEAD 4f5ad5a1`, all four ROMs present.

```
378 suites · 4,351 passed · 0 failed · 2 ignored · CARGO_EXIT=0
```

Baseline is 378 / 4,348 / 0 / 2, so the delta is **+3 passed** — exactly the three
tests this parcel adds. The two ignored are the pre-existing
`sigil_diff_reports_byte_identity` and `secondary_pin_classes_match_the_hand_typed_baseline`.
The log's three `panicked at` lines are tests that deliberately assert a panic,
surfaced by `--nocapture`; `failed=0` across all 378 result lines.

**Clippy.** `cargo clippy --release --workspace --all-targets -- -D warnings`,
**exit 0**. The 19 `warning:` lines are all vendored C++ build-script output from
`sigil-clownlzss-sys`; no Rust lint fired.

**Aeon byte-neutrality.** All four ROMs deleted first, then one shape per
invocation with `SIGIL_VERSION_STRICT=1` against `/home/volence/sonic_hacks/.aeon-eval-ref`
(detached at `4f5ad5a1`), each `rc=0`:

| shape | crc32 | size | moved? |
|---|---|---|---|
| `s4.bin` | `14ee2440` | 719,700 | no |
| `s4.debug.bin` | `142294b3` | 737,683 | no |
| `demo.bin` | `0c456778` | 96,474 | no |
| `demo.debug.bin` | `2e603d53` | 101,339 | no |

**No shape moved.** That is the expected result and the reason is structural, not
luck: aeon's Z80 is declared in `.emp` as `module … (cpu: z80)` / `section …
(cpu: z80, …)` and lowered by the `.emp` front end, and its residual `.asm` root
carries no `CPU Z80` at all — so the AS front-end path this parcel changes is
unreachable from aeon. `golden/`, `pins.rs` and `repin.toml` were not touched and
nothing was refrozen.
