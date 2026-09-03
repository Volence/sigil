# `align` under asl 1.42 Beta [Bld 212] — the measured rule

Flags: `-xx -n -q -A -L -U -i .` (the corpus flags).
Runner: `./run.sh <probe>`; single-case sweeps: `./gen_org.sh <org-expr> <n>`.

## Which binary, and why that is written down here

A version string is not a binary identity. Two DIFFERENT Linux-x86_64 asl builds
sit in this workspace and both self-report `AS V1.42 Beta [Bld 212]`:

| build | banner | md5 |
|---|---|---|
| `s2disasm/build_tools/Linux-x86_64/asl` — flamewing fork | `(x86_64-Linux)`, `(C) 2022-2024 flamewing` | `0dee1f98e6480a4783d27ffd8b90896f` |
| `s1disasm/build_tools/Linux-x86_64/asl` — upstream | `(x86_64-unknown-linux)`, MPC821 additions | `61e672562465725a8c102288a7da9098` |

`skdisasm` and `sonic_hack` carry the same upstream binary (`61e672…`).

**They AGREE on every row that discriminates the rule**, so the fork is not the
variable behind the July/September disagreement. Run `./gen_org_both.sh <org> <n>`
to reproduce; the sweep taken 2026-09-03:

```
$0000B000    n=256    s2/flamewing=B000         s1/upstream=B000         agree
$0000B02A    n=256    s2/flamewing=B100         s1/upstream=B100         agree
$0000B02A    n=100    s2/flamewing=B02C         s1/upstream=B02C         agree
$0000B02A    n=2      s2/flamewing=B02A         s1/upstream=B02A         agree
$7FFFB02A    n=256    s2/flamewing=7FFFB100     s1/upstream=7FFFB100     agree
$FFFFB000    n=256    s2/flamewing=FFFFB100     s1/upstream=FFFFB100     agree
$FFFFB001    n=256    s2/flamewing=FFFFB100     s1/upstream=FFFFB100     agree
$FFFFB002    n=256    s2/flamewing=FFFFB200     s1/upstream=FFFFB200     agree
$FFFFB005    n=256    s2/flamewing=FFFFB200     s1/upstream=FFFFB200     agree
$FFFFB026    n=256    s2/flamewing=FFFFB200     s1/upstream=FFFFB200     agree
$FFFFB100    n=256    s2/flamewing=FFFFB200     s1/upstream=FFFFB200     agree
$FFFFB102    n=256    s2/flamewing=FFFFB300     s1/upstream=FFFFB300     agree
$FFFFB02A    n=100    s2/flamewing=FFFFB0B4     s1/upstream=FFFFB0B4     agree
$FFFFB02A    n=2      s2/flamewing=FFFFB02C     s1/upstream=FFFFB02C     agree
$FFFFFF00    n=256    s2/flamewing=100000000    s1/upstream=100000000    agree
$80000000    n=256    s2/flamewing=80000100     s1/upstream=80000100     agree
```

`p1` (the source the July test transcribes) also answers `$B100` under both.

Every listing row below was taken with the flamewing build `0dee1f98…`, and the
four rows the 2026-07-08 note recorded were re-taken under BOTH.

## The rule

`align n` moves the program counter by a delta computed on the **low 32 bits of
the PC read as a signed 32-bit integer**, with C's truncating remainder:

```
t32   = (int32) (pc + n - 1)          // low 32 bits, signed
a32   = t32 - (t32 % n)               // C '%': remainder takes the DIVIDEND's sign
delta = (uint32) (a32 - (int32) pc)   // unsigned 32-bit difference
pc'   = pc + delta                    // added to the WIDE pc, so it can exceed 32 bits
```

For a PC whose low 32 bits are **non-negative** this is the plain round-up:
already-aligned is a no-op. For a PC whose low 32 bits are **negative**
(`$8000_0000`..`$FFFF_FFFF` — i.e. every 68k RAM address) the truncating `%`
rounds toward zero, so the result lands one block *high* for most offsets, and
an already-aligned negative PC advances a full block.

`n` is taken modulo 2^16 (a `Word`): `align -256` behaves as `align $FF00`.
`align 0` makes asl **abort with SIGFPE** (integer divide by zero).
`align 1` is a no-op everywhere.

## Listing rows

`phase` (PC displayed sign-extended to 64 bits):

```
p1   3/    B000 :   phase $B000        p3   3/    B040 :   phase $B040
     4/    B000 :   ds.b 5                  4/    B040 :   align 256
     5/    B005 :   align 256               5/    B100 : B100  L: dc.w L
     6/    B100 : B100  L: dc.w L

p4   3/    B000 :   phase $B000        p5   5/    B005 :   align 256
     4/    B000 :   align 256               6/    B100 : B100  L: dc.w L
     5/    B000 : B000  M: dc.w M           7/    B102 :   align 256
                                            8/    B200 : B200  N: dc.w N

p6   8/FFFFFFFFFFFFB026 :   ds.b 4
     9/FFFFFFFFFFFFB02A :   align 256
    10/FFFFFFFFFFFFB200 :   Player_Pos_Ring: ds.b 256      <-- the extra block

p7   3/FFFFFFFFFFFF0000 :   phase $FFFF0000
     4/FFFFFFFFFFFF0000 :   align 256
     5/FFFFFFFFFFFF0100 : X: dc.w X                        <-- aligned, still moved
```

`org` (one isolated assembly per row — repeated `org`+`align` in one file is
fine, but isolate anyway so nothing is an ordering artifact):

| org | n | asl | signed-round-up would be |
|---|---|---|---|
| `$0000B000` | 256 | `B000` | `B000` |
| `$0000B001` | 256 | `B100` | `B100` |
| `$0000B02A` | 256 | `B100` | `B100` |
| `$0000B02A` | 100 | `B02C` | `B02C` |
| `$7FFFB02A` | 256 | `7FFFB100` | `7FFFB100` |
| `$FFFFB000` | 256 | **`FFFFB100`** | `FFFFB000` |
| `$FFFFB001` | 256 | `FFFFB100` | `FFFFB100` |
| `$FFFFB002` | 256 | **`FFFFB200`** | `FFFFB100` |
| `$FFFFB003` | 256 | **`FFFFB200`** | `FFFFB100` |
| `$FFFFB0FF` | 256 | **`FFFFB200`** | `FFFFB100` |
| `$FFFFB02A` | 256 | **`FFFFB200`** | `FFFFB100` |
| `$FFFFB100` | 256 | **`FFFFB200`** | `FFFFB100` |
| `$FFFFB101` | 256 | `FFFFB200` | `FFFFB200` |
| `$FFFFB102` | 256 | **`FFFFB300`** | `FFFFB200` |
| `$FFFFB000` | 100 | **`FFFFB0B4`** | `FFFFB000` |
| `$FFFFB02A` | 100 | **`FFFFB0B4`** | `FFFFB064` |
| `$FFFFB02A` | 3 | `FFFFB02C` | `FFFFB02C` |
| `$FFFFFE00` | 256 | **`FFFFFF00`** | `FFFFFE00` |
| `$FFFFFF00` | 256 | **`100000000`** | `FFFFFF00` |
| `$FFFFFF01` | 256 | `100000000` | `100000000` |
| `$FFFFFFFF` | 256 | `100000000` | `100000000` |
| `$80000000` | 256 | **`80000100`** | `80000000` |
| `$80000001` | 256 | `80000100` | `80000100` |
| `$0000B02A` | -256 | `FF00` | — (`n` is a `Word`: `$FF00`) |

Every row above is reproduced by the formula, including the two that exceed 32
bits: the delta is an unsigned-32 difference but is added to a wide PC.

## The four rows the 2026-07-08 note recorded

The July comment lists `$B000→$B100`, `$B005→$B200`, `$B026→$B200`,
`$B100→$B200` and calls the regime "inside a `phase`". All four reproduce today
as RAM addresses under both binaries, and none of them reproduces at the
positive `phase $B000` the regression test transcribed them to:

```
$FFFFB000    n=256    s2/flamewing=FFFFB100     s1/upstream=FFFFB100
$FFFFB005    n=256    s2/flamewing=FFFFB200     s1/upstream=FFFFB200
$FFFFB026    n=256    s2/flamewing=FFFFB200     s1/upstream=FFFFB200
$FFFFB100    n=256    s2/flamewing=FFFFB200     s1/upstream=FFFFB200
```

July measured correctly and recorded the addresses by their low half. What was
lost is that they were negative PCs — not that they were phased.

The rule generalised from them, `round_up(pos + n, n)`, is wrong on both sides:
it disagrees with asl on 20 of the 31 rows here, including 5 of the 11 RAM rows.
