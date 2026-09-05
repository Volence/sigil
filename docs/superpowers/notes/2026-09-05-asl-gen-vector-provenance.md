# The golden vectors now say which asl answered — and what that still does not prove

2026-09-05. Parcel ASL-GEN-VECTORS-UNIDENTIFIED, branch
`parcel/asl-gen-vector-provenance`.

Ground truth for everything below: reference build **md5
`61e672562465725a8c102288a7da9098`**, varying build **md5
`0dee1f98e6480a4783d27ffd8b90896f`**, `p2bin` **md5
`4f2fff99c3347bafb93b12d5be1db754`** — the same file in both installs, so a
cross-build differential isolates `asl`. Cited by digest, never by banner
(`asl-reference/README.md`).

## The defect

Three generators mint the committed reference vectors by shelling out to a real
`asl`:

| generator | golden | entries |
|---|---|---|
| `crates/sigil-isa/src/bin/gen_m68k_vectors.rs` | `crates/sigil-isa/tests/m68k_golden_vectors.txt` | 176 |
| `crates/sigil-isa/src/bin/gen_z80_vectors.rs` | `crates/sigil-isa/tests/z80_golden_vectors.txt` | 120 |
| `crates/sigil-frontend-as/src/bin/gen_snippet_vectors.rs` | `crates/sigil-frontend-as/tests/snippets_golden.txt` | 227 |

They take `ASL_BIN`/`P2BIN_BIN` from the environment with no default and no
digest check — confirmed from source, and the goldens carried **no provenance
line of any kind**. Those files are the frozen independent-asl witness six test
targets compare against forever. A witness whose whole value is "an independent
implementation said so" could not say which implementation said so.

## The ruling: STAMP, DO NOT CONSTRAIN

Pinning one digest was rejected. `tools/asl` was deliberately deleted at the P4d
flip (kill-list row 96); the `ASL_BIN` hook exists so someone with their own AS
install can extend the corpus out-of-repo. A pin would fight that decision rather
than compose with it.

So each generator now **derives** the identity of whatever it was handed and
records it, and **refuses to write a golden it cannot stamp**. That identifies
the instrument at the point of use without dictating which one — which is what
the week's finding actually says the defect was: *not a second build, an
unidentified instrument.*

`crates/sigil-isa/src/asl_provenance.rs` is the shared helper. `sigil-frontend-as`
does not depend on `sigil-isa` and neither crate carries third-party
dependencies, so it is shared by `#[path]` include — the same device the
generators already use for their corpus modules — and MD5 is implemented inline
(RFC 1321) rather than shelling out to `md5sum`, because a stamp that can
silently not happen is the defect this closes.

### The header, and why it cannot be read as a correctness claim

It opens `PROVENANCE — generated. This records WHICH BUILD ANSWERED.` and then
**prohibits** the misreading rather than hedging it: *"Do NOT read it as a claim
that the answer is correct, and do not cite it as one."* It gives the mechanism —
asl substitutes a stable value for an operand it declines to evaluate, so
re-derivability is evidence about the INSTRUMENT and never about the vectors —
and points at the probe directory for the question a digest cannot answer.

Fields, all derived per run: `minted-by`, `asl-md5`, `asl-banner` (×2),
`p2bin-md5`, `p2bin-banner`.

Two measurements behind the shape. **asl prints its banner on stdout**, not
stderr (311 lines of help, exit 1 with no args); the first two lines are the
version. **p2bin prints no version string at all** — its no-arg output is a usage
message — so its row says so, and the digest is its only identity, which is the
general case stated plainly rather than a blank.

**No install path in the file.** A path is not a property of the binary; writing
it in would churn the header between two machines holding the byte-identical
build, training readers to ignore exactly the diff worth having. Paths go to
stderr for whoever runs the mint.

### Derived at run time, enforced not asserted

`no_digest_literal_in_this_source` fails if any 32-hex-digit literal appears
outside the RFC test block. This is the **inverse** of the guard in
`asl-reference/`, where the wanted digest IS written out: that is a GATE and its
expectation must not move; this is a STAMP and its value must.

Red-first: with `pub const REFERENCE_ASL_MD5: &str = "61e6725624…";` on disk at
line 35 (confirmed by grep — the file was untracked at that point, so `git diff`
was silent and is *not* the application proof), the test failed naming line 35
and that literal, and it alone of the four went red. Restored, 4/4 green.

Also: 7/7 RFC 1321 published vectors; two files differing by one byte stamp
differently; unreadable and empty binaries both refuse with a reason.

### The refusal, end to end

Three cases, each exiting 4 with the golden's md5 unchanged before and after —
the refusal lands before anything is written, because provenance is captured
before minting begins:

| ASL_BIN / P2BIN_BIN | message |
|---|---|
| a text file marked executable | `cannot run … to read its banner: Exec format error` |
| `/bin/true` (runs, prints nothing) | `printed no banner on stdout — refusing to stamp a tool it cannot describe` |
| real asl, empty p2bin | `… is empty — refusing to stamp it` |

### What the header does to the no-op invariant — the demonstration

Re-minting all three under the reference build is still a git-clean no-op. Under
the **varying** build, `z80_golden_vectors.txt` changes **no vector line** — all
120 identical — and changes only the digest and the banner's *second* line. The
first banner line is the same on both. **Before this change that mint was a
clean no-op**, indistinguishable from a reference-build mint. The header did not
add a dependency; it made a silent one visible.

## The audit: what the EXISTING committed vectors are worth

**All three re-mint byte-identical.** Reproduced two independent ways.

The committed sweeps (`2026-09-05-asl-nondeterminism-sweep-probes/`), N=3:

| golden | committed md5 | verdict |
|---|---|---|
| `m68k_golden_vectors.txt` | `804f4f5370a7ac5fb77554c2504e6a4e` | STABLE across 3 runs AND identical to committed |
| `z80_golden_vectors.txt` | `0b3d8455c82127b321c950d554117ab5` | STABLE across 3 runs AND identical to committed |
| `snippets_golden.txt` | `a0318bd39f40af8eccd7e0c540c64ece` | STABLE across 3 runs, 227/227 blocks, 0 differing |

And when the stamp was applied, the body of each file was diffed against
`git show HEAD:` with the header stripped: **176 / 120 / 2163 lines byte-identical.**
So the stamp went onto vectors *proven* to be the committed ones, not onto a
fresh mint swapped in behind it. That is why the audit ran first.

Restores: the generators rewrite in place, so every run is followed by
`git checkout --` on the three paths and the restore is **verified** with
`git status --porcelain` before any verdict is drawn; the new sweep additionally
refuses to start if the golden paths are not already clean.

**What this establishes and what it does not.** It establishes that the committed
vectors are re-derivable under a known instrument, so the stamp is applied with
evidence rather than assumption. It does **not** establish that the reference
build is what originally minted them — byte identity is silent on provenance, and
that is unrecoverable for the pre-stamp history. It is now recorded going
forward, which is the whole point.

## The declined-operand enumeration

The question underneath: **does any committed vector quote a byte column for a
shape asl DECLINES to value?** Banner-vs-digest is blind to this class.

**Population: all 523 committed entries. Three parameters, all agreeing, 0 found.**

### Parameter 1 — cross-build disagreement (`declined_operand_sweep.sh`)

The varying build fills a declined operand from uninitialized memory, so an entry
that differs between the builds, or varies across N varying-build runs, carries
one. N=4:

| golden | entries | DRIFT | DIFFERS | UNSTABLE |
|---|---|---|---|---|
| `m68k_golden_vectors.txt` | 176 | 0 | 0 | 0 |
| `z80_golden_vectors.txt` | 120 | 0 | 0 | 0 |
| `snippets_golden.txt` | 227 | 0 | 0 | 0 |

Red-first: `INJECT=1` appends a block of the *silent* class (`#f(<register>)`,
exit 0, no diagnostic). Proven to land on disk (`git diff --stat` = 1 file, +7),
and it fired with four distinct uninitialized draws —
`30 3C 55 86 / 55 AC / 55 FC / 56 36`. A clean verdict from this sweep is
therefore a result, not a blind spot.

### Parameter 2 — did asl say anything (`diagnostics_sweep.sh`)

Needs no second build. Control first: the loud refusal `move.w #65536,d0` must be
flagged (it was, "exit 2") or the sweep exits 4 rather than reporting.

**15 complaints over 523 entries, and none is a declined operand.** Each was
adjudicated against its committed bytes, not counted:

| n | complaint | verdict |
|---|---|---|
| 11 | `privileged instruction` — `rte`, `move.w #$2700,sr`, `move.l {a0,a6,a7},usp`, `move.l usp,{a0,a7}`, + 4 snippet blocks | supervisor-mode advisory about the *instruction*. Encodings canonical: `4E73`, `46FC 2700`, `4E60/4E66/4E67`, `4E68/4E6F`. Operand fully valued. |
| 2 | `bit number will be truncated` — `bchg #255,(a0) => 08 50 00 FF` | **the decisive one.** asl emits `FF` VERBATIM; it substituted and truncated nothing. The warning is that 68000 hardware takes a memory-operand bit number mod 8 at RUN time. This is the site that most reads like a decline and is not one. |
| 1 | `as_warning_is_byte_inert` | the block's own `warning` directive; the block exists to assert it is byte-inert (`11 22`). |
| 1 | `as_warning_and_exitm_the_clearram_shape` | the macro's own designed warning+`exitm` under test (`11 C0 22 33`). |

**Its blindness is measured, not assumed.** The silent shape run through this
sweep's own instrument: exit 0, stdout 0 bytes, stderr 0 bytes, listing
`0 errors / 0 warnings` — and it emits `303C 0000`, a value asl never computed.
That is why there are two parameters and not one.

### Parameter 3 — structural, and the strongest result here

The two arrow corpora **cannot express the declined shapes at all**:

- The **loud** class is closed by the generators' own
  `assert!(asl_out.status.success())` — an exit-2 refusal panics the mint, so no
  such entry can reach the file. (Every mint in this parcel succeeded.)
- The **silent** class needs a `function` directive on its own line. Both corpus
  modules contain **zero** multi-line snippets, and both templates are a fixed
  `cpu`/`org` plus exactly one snippet line. There is no vehicle.

The snippet corpus *does* have the vehicle — three `function` definitions across
two blocks — so those were read by hand. Every call carries a numeric or string
argument, never a register, and each value checks out arithmetically:
`dsp(k)+2 => 01 4F` = (11×7)+$100+2; `#dsp(k) => 01 4D`; `dc.w id($1C6) => 00 C9`
= (454−64)/2+6. The `dsp(a1)` / `id(a1)` occurrences are displacements in memory
operands, not immediates, and resolve to the equates' values (`00 2A`, `00 00`).

## What I concluded was wrong in the brief

1. **"There is no equivalent for the m68k and z80 vector files — that gap is part
   of this parcel."** There is:
   `2026-09-05-asl-nondeterminism-sweep-probes/sweep_isa_vectors.sh`, committed,
   which does the re-mint-N-times comparison for both. I used it rather than
   rebuild it. The gap that *was* real is the declined-operand enumeration, which
   existed for no corpus; that is what the new probe directory adds.
2. **"both banner lines"** for p2bin — p2bin has no banner. Handled explicitly.

## Left open (not done here)

- **374 workspace test rows are unmeasured in this worktree**, not green. No aeon
  reference tree is provisioned, and the harness refuses loudly rather than
  reporting zero. Under `SIGIL_ALLOW_PARTIAL=1` the workspace is 4550 passed / 0
  failed / 395 targets, and 4176 + 374 = 4550 confirms the refused rows are
  exactly the reference-dependent ones. **None is in `sigil-isa` or
  `sigil-frontend-as`**, both of which are fully measured and green. Provisioning
  the reference tree and re-running is a foreground follow-up.
- **The five 2026-09-03 notes** booked in `asl-reference/README.md` as citing the
  varying build by banner are still un-re-measured. Untouched by this parcel.
- **`operands.rs:519`**'s version-string citation, booked in the same place.
