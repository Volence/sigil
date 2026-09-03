# A reservation advances the write cursor, so its gap fills like p2bin

`ds` reserved address space and left the image write cursor alone, so anything
after a reservation inside one section packed short of its own address. asl
reserves by leaving a GAP in the object file and `p2bin` fills that gap for
whatever follows; the two models agreed only while the reservation trailed.

asl 1.42 + p2bin against sigil-before-the-fix, same source, one section:

```text
  asl:   11 00 00 00 00 22 00 00 00 00 00 00 33 00 00 00 00 00 00 00 00 44 00 00 00 16
  sigil: 11 22 33 44 00 00 00 16
```

Both assemblers agreed the trailing label was `$16`; only asl's image had a byte
there. **Exit 0, no diagnostic, twenty bytes short.** The diagnostic count is
structurally blind to this shape, and so were both corpora — see below.

## The rule

The write cursor advances over a `Reserve` and the image grows only where
something writes. That is both halves of p2bin's rule at once:

- a reservation with something after it inside the section is part of the image
  — the next write fills the gap;
- a **trailing** reservation is trimmed, because nothing writes past it;
- a section that is nothing but reservations — Aeon's phased `$FFFF….` RAM
  regions — still places no byte.

Three image walks carry the rule and all three move together:
`Section::image_bytes` (`sigil-ir`), `link`'s fixup-offset replay, and
`image_final_size` (`sigil-link/relax.rs`). The VMA walks
(`final_size`/`placement_span`/`vma_len`) already counted the reservation and are
untouched.

## What `image_final_size` decides, and the hole under it

`image_final_size` feeds the post-fixpoint **overlap check**. The extent it
reports is the range a section is held to own, so understating it does not
produce a wrong image directly — it produces a **missed collision**. A section
that is `$11`, a four-byte `ds`, then `$22` owns `[lma, lma+6)`; read as
`[lma, lma+2)` a pin at `lma+4` intersects nothing, and `flatten` lays that
section's bytes over the reserved range with nothing said about it. The silent
clobber arrives through the check built to refuse it.

The arm had no gate at all: reverting it to the address-only model survived the
entire workspace suite, 4,326 passed and 0 failed, with the mutation on disk.
`a_reservation_inside_a_section_counts_toward_its_overlap_extent` closes it.

## The corpora are blind to this by construction

| corpus | before | after |
|---|---|---|
| Sonic 1 — `s1disasm` `f6ece657`, entry `sonic.asm` | 1,367 | 1,367 |
| Sonic 2 — `s2disasm` `e45ebf3`, entry `s2.asm` | 9,625 | 9,625 |

Not merely equal in total: the two diagnostic streams are **byte-identical**,
both directions, so the per-class decomposition and the unresolved-symbol sets
are identical by construction rather than by comparison.

They cannot move. Both corpora fail in the FRONT END and exit 1 before
`sigil_link::link` is called at all, and this parcel changes nothing but image
emission. A corpus count is not a witness for an image rule, which is the same
blindness the twenty-byte-short image exploited.

## Aeon

All four shapes rebuilt from deleted artifacts, one shape per invocation, at
`4f5ad5a1` with `SIGIL_VERSION_STRICT=1`:

| shape | CRC32 | size |
|---|---|---|
| `s4.bin` | `14ee2440` | 719,700 |
| `s4.debug.bin` | `142294b3` | 737,683 |
| `demo.bin` | `0c456778` | 96,474 |
| `demo.debug.bin` | `2e603d53` | 101,339 |

**Byte-identical, all four.** Aeon has no `.asm` `ds` with image bytes after it
in the same section; its reservations are RAM regions, which the trailing/pure
halves of the rule leave alone.

## Verification

Every expected value below is read off `p2bin`'s own output —
`asl -xx -n -q -A -L -U -i .` then `p2bin <probe>.p <probe>.bin` — not off
sigil's.

Three gates:

- `a_reservation_fills_like_p2bin_and_trims_like_p2bin` — the three source-level
  shapes: gap-filled, trailing-trimmed, phased-RAM.
- `a_fixup_after_a_reservation_lands_behind_the_gap` — pinned at the IR level on
  purpose. Written as AS source a same-section label is FOLDED by the front end,
  so the line carries no fixup and a source-level fixture cannot reach `link`'s
  replay walk; a mutation reverting that walk reads green through one.
- `a_reservation_inside_a_section_counts_toward_its_overlap_extent` — the
  overlap extent, with a non-colliding control so it cannot pass on an unrelated
  refusal.

Red-first, three mutations, each shown applied from disk and restored from a
committed baseline:

| mutation | red |
|---|---|
| `image_bytes`'s `Reserve` arm back to `{}` | 9 tests across 3 crates |
| `link`'s replay `Reserve` arm back to `{}` | 1 — the IR-level fixup gate |
| `image_final_size`'s `Reserve` arm back to `{}` | 1 — the overlap gate |

The third was **applied-and-still-green** across the full workspace before that
gate existed; chasing it is what found the missed-collision hole.

Seven expectations encoded the packed model and were rewritten, not deleted.
Three align probes now assert p2bin's size as well as its trailing bytes
(`assert_p2bin_image`): p1 258 bytes ending `B1 00` at `$100`, p9 45,570 ending
`B2 00` at `$B200`, p11 45,102 ending `B0 2C`, p12 44 ending `B0 2A`. The DS.B
row of `directive_keywords_fold_at_every_recognition_site` is `01 00 00 03`; the
two `move_l_imm_*` tests measure from `DS_L_1_GAP` because `RamPtr: ds.l 1` opens
a gap the instruction fills; `guarded_define_folds_identically_to_in_file_equate`
folds to 102 bytes rather than 6, and the fold IDENTITY it exists to prove is
untouched either way.

Clippy `--release --workspace --all-targets -- -D warnings`: exit 0.

### The runs

Both are `scripts/landing-run.sh`, same reference tree (`.aeon-eval-ref` at
`4f5ad5a1`, all four ROMs present), each in its own on-disk target directory.

| | tree | suites | passed | failed | ignored | exit |
|---|---|---|---|---|---|---|
| master baseline | `.sigil-reserve-base` @ `60320df4` | 377 | 4,324 | 0 | 2 | 0 |
| this parcel | `.sigil-reserve-land` @ `dd41c8ff` | 377 | 4,327 | 0 | 2 | 0 |

`4,324 baseline + 3 new = 4,327`, and the three are the three gates above —
each named to `--expect-test`, so a green log that did not run them would have
been refused rather than believed.

One earlier attempt read `FAILED — 1 test red` on
`version_reports_the_head_of_the_tree_it_was_built_from`. The cause was a commit
landing while that run was in flight: the binary was baked at `7b9bae69` and the
checkout reached `dd41c8ff` under it. The gate's own message names that case and
says to re-run to distinguish it from a build.rs trigger failure; the re-run at
a stable tip is the row above. Do not commit into a landing run.
