# Probes for `irp` / `irpc` / `ARGCOUNT`

The measurements behind `../2026-09-03-irp-irpc-argcount.md`. Committed because
four of them are the **falsifier for a finding that was booked and not fixed**,
and a falsifier that lives in a scratch worktree has a lifetime of one session.

## The oracle

`s1disasm/build_tools/Linux-x86_64/asl`, md5
`61e672562465725a8c102288a7da9098` — **upstream AS, not the flamewing fork
`s2disasm` ships under the same version string.** Every row in the note was
measured with this one.

Flags, taken from `s1disasm/build_tools/lua/common.lua:773`:

```sh
asl -xx -n -q -A -L -U -E -i . <probe>.asm      # writes <probe>.lst and <probe>.log
```

`-U` is not optional. A rule measured without it has been wrong here before.

## Running one

`diff_bytes.sh <probe>.asm` assembles with **both** tools, converts asl's `.p`
with `p2bin`, and compares the images by CRC32 + size. It prints both exit codes,
because a probe where sigil exits 0 and asl refuses reads like agreement if you
only look at one of them — that happened during this parcel.

```sh
SIG=/path/to/target/release/sigil ./diff_bytes.sh p1.asm
```

## What each one measures

| probe | subject |
|---|---|
| `p1` | `irp`/`irpc` basics; `endr` as a closer. **Byte-identical to asl.** |
| `p2` | `ARGCOUNT` under `shift`, first cut — confounded by the `dc.b`/parameter-`b` collision, kept because that collision is itself instructive |
| `p3` | `ARGCOUNT` × (1, 3 parameters) × (0, 1, 3, 5 arguments), 5 shifts each |
| `p4` | `ARGCOUNT` × (2, 4 parameters) × (0, 1, 2, 4, 6 arguments), 7 shifts each |
| `p5` | `ARGCOUNT` for empty and elided argument fields; the `jmpTos` `ALLARGS` relay |
| `p6` | empty lists, string-symbol operands, undefined operands, whitespace, the substitution boundary, nesting |
| `p7` | `irpc` on `""` and on an empty field; quoted commas; escapes; concatenation; case sensitivity; no leak after the loop |
| `p8` | integer `irpc` operands; `irp` items as raw text; loop in a macro; `shift` inside a loop body |
| `p9` | `ARGCOUNT` boundary/folding/parameter-shadowing; a head with no list |
| `pe` | `dc.b ""` in isolation — the control that showed `pf`'s two bytes are NOT this |
| **`pf`, `pg`** | **`shift` leaves AS's internal placeholder `\001\00N` in a vacated parameter slot, not an empty string.** Falsifier for finding (d) |
| `pj` | the whole S2 `jmpTos` chain end to end |
| `pk` | the same chain reduced to what sigil can run, for reading the values back |
| **`pn`** | **`~~` is asl's LOGICAL NOT; sigil computes double bitwise NOT.** Falsifier for finding (b) — one command |
| `pu` | an undefined symbol in an `equ`, which sigil DOES catch (at link) |
| **`pv`** | **`NAME := <undefined>` binds 0 silently.** Falsifier for finding (c) |

## The demo byte sweep

`mkharness.sh` builds one standalone `.asm` per S1 demo script, wrapping the
corpus's own `demoinput` macro (`demoinput.inc`, copied verbatim from
`Macros.asm:310-341`) around the corpus's own data. It needs a checkout of
`s1disasm` at `f6ece657`; edit the `S1=` line to point at one.

All 13 are byte-identical to asl. The sweep is **proven able to fail** — see the
note's mutation list, and `mutate.sh` for the shape.

## The rest

- `mutate.sh` — red-first proof driver: applies a mutation to a **committed**
  baseline, refuses if its anchor did not match exactly once, prints the patch
  read back from disk, runs the named tests, restores.
- `classify.sh` / `delta.py` — per-class before/after decomposition of a sigil
  diagnostic log, asserting the rows sum to the raw line count.
