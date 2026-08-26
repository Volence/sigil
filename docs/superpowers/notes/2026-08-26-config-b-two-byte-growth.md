# The two bytes `config_b` gained across the ROM re-layout

**Question.** Between chain entry 167 (`showcase-effects`, master `aa641667`) and 168
(`rom-relayout`, master `b225eddc`), `config_b` — the **sound-OFF** shape — grew by exactly
two bytes, `full_size` 610359 → 610361, under a parcel whose entire subject was moving the
Z80 sound banks. The suspicion put to this investigation was **R7** of
`2026-08-26-placement-constraint-inventory.md`: alignment inferred from a section's own
frozen base, so a repin can silently change a quantum and open two bytes of pad.

**Answer: R7 is not the cause, and nothing in the assembled image changed size at all.**
The two bytes are in the `convsym` `deb2` debug-symbol appendix that follows `EndOfRom`.
They are one additional per-chunk block header, and they exist because the re-layout moved
`DPLC_Sonic` and `Art_Sonic` into the `$70000–$7FFFF` window, which previously held no
symbol. Benign, and bounded at ≤ 512 bytes by construction.

The aeon report's **"44 rows moved, 0 alignment-quantum changes" survives**, re-derived
here independently. It was never in tension with the growth: the two statements are about
disjoint halves of the file.

---

## 1. The assembled image did not change size

`EndOfRom` is where `append_deb2_appendix` puts the `de b2` magic, so the magic's offset
*is* the assembled length. It is identical on both sides:

```
$ python3 - <<'EOF'
old = open('old_config_b.bin','rb').read(); new = open('new_config_b.bin','rb').read()
print(len(old), len(new), hex(old.find(b'\xde\xb2', 0x8b000)), hex(new.find(b'\xde\xb2', 0x8b000)))
EOF
610359 610361 0x8b6f0 0x8b6f0
```

Two more independent witnesses agree: `# assembled_end=0x8b6f0` is unchanged in
`golden/offcanonical_sizes/config_b.txt` across the diff, and `anchor_end = 0x8b6f0` is
unchanged in both `provenance.toml` entries. `config_b` is the **only** shape with an
unchanged `anchor_end` — every other non-demo shape grew `+0x49d0` in the image, which is
why only this one had a two-byte residual left over to explain.

So: **no placement pad was inserted anywhere.** Whatever R7 might do, it did not do it here,
because there is no room in the image for it to have done it.

## 2. What the appendix is, and the size model

`native::append_deb2_appendix` shells `<aeon>/tools/convsym … -output deb2 -a`, which
appends the MD Debugger's address→name table after `EndOfRom`. The format is undocumented
in this tree, so it was reverse-engineered by feeding the real `convsym` controlled
`as_lst` listings (`sigil_link::emit_listing`'s shape) and reading the bytes back:

```
appendix = 4                     magic + u16 header (= 4*n_chunks + 2)
         + 4 * n_chunks          one u32 per 64 KB window of the 24-bit address space;
                                 0 means "no symbol lives in this window"
         + 4 * n_huffman_leaves  (u16 code, u8 bit-length, u8 char) canonical-Huffman table
         + 2                     the ff ff sentinel before the first block
         + SUM over NON-EMPTY chunks of
               2                   the block's u16 header (offset to its blob)
             + 4 * records         (u16 addr-low, u16 byte offset into the blob)
             + blob                align_up(SUM ceil(bits(name)/8) + 1, 2)
                                   — the LAST chunk takes the +1 guard byte with no word pad
```

`n_chunks` is `(highest address >> 16) + 1`, pinned at 256 by the kept `$FFFFxxxx` RAM
labels. A chunk-table entry points **four bytes before** its block header.

Decoder committed alongside this note as `assets/deb2_appendix.py`. It is an investigation
artifact and **not a gate** — nothing runs it, it asserts nothing, and a clean run of it is
not a check having passed.

The model predicts the byte-exact length of **all twelve** shipped appendices (six shapes ×
two chain entries; `lean` carries none — its file size equals its `EndOfRom`):

| shape | entry 167 | entry 168 | non-empty chunks | records | Σ⌈bits/8⌉ |
|---|---|---|---|---|---|
| config_a | 0xbaba ✓ | 0xbabc ✓ | 9 → 9 | 2448 → 2448 | 0x8f52 → 0x8f52 |
| config_b | 0x9947 ✓ | 0x9949 ✓ | **7 → 8** | 2008 → 2008 | 0x74c4 → 0x74c4 |
| demo | 0x6658 ✓ | 0x6658 ✓ | 3 → 3 | 1299 → 1299 | 0x4cff → 0x4cff |
| demo_debug | 0x78bc ✓ | 0x78bc ✓ | 3 → 3 | 1538 → 1538 | 0x5ba7 → 0x5ba7 |
| s4 | 0x9bf5 ✓ | 0x9bf5 ✓ | 9 → 9 | 2052 → 2052 | 0x76bc → 0x76bc |
| s4_debug | 0xb93e ✓ | 0xb940 ✓ | 9 → 9 | 2429 → 2429 | 0x8e22 → 0x8e22 |

**The load-bearing column is the last two.** Record count and total Huffman-coded name
length are identical on both sides of every shape, and the 65-leaf code table is
byte-identical too (`old[trie] == new[trie]` → `True` everywhere). Those are the
placement-*independent* terms: the re-layout permuted addresses, it did not add, remove or
rename one symbol. **The only way placement can reach the appendix's size at all is by
changing how the symbols partition across the 64 KB windows.**

## 3. The two bytes, exactly

For `config_b` the partition changed by gaining a window:

```
OLD  non-empty chunks: 00 01 02 04 06 08 ff   (7)
NEW  non-empty chunks: 00 01 02 04 06 07 08 ff (8)
```

and every other term held constant, including the name-blob total:

```
       magic  chunktab  huffman  ffff  blockhdrs  records  name blobs
old:     4  +  1024   +   260   +  2  +    14   +  8032  +  29903  = 0x9947  (actual 0x9947)
new:     4  +  1024   +   260   +  2  +    16   +  8032  +  29903  = 0x9949  (actual 0x9949)
                                          ^^^^ 2*7 -> 2*8, the ONLY term that moved
```

(29903 = `0x74cf` is the blob total *including* its per-chunk guard/pad bytes; the
placement-independent `Σ⌈bits/8⌉` underneath it is `0x74c4` on both sides.)

The marginal two bytes are **chunk `07`'s block-header word**, at appendix offset `0x8676`,
ROM offset **`0x93d66`**, value `00 0a` (= "blob starts 10 bytes on", i.e. two records).
The two symbols that made that window non-empty:

```
0x071870  DPLC_Sonic
0x0721b0  Art_Sonic
```

Everything after `0x93d66` is the same bytes shifted two forward — chunk `ff`'s block (the
RAM labels, 4287 bytes) is byte-identical modulo the shift, and chunks `00`/`01` are
byte-identical unshifted.

`config_a` and `s4_debug` also gained +2 in the appendix, from the *same* mechanism in a
different term: their non-empty chunk count was unchanged, but the partition still moved, and
the per-chunk `align_up(…, 2)` slack went 12 → 14 bytes. Same cause, different arm.

## 4. Only 18 symbols moved, and the exact landing at `0x2a3c0` is by construction

The frozen table's 68 rows are a coarse net, and reading "only `HeightMaps` and `Map_Tails`
moved" off it understates the parcel — but not by much. Decoding both appendices and
diffing all 2008 symbols gives **18** moved, in exactly two rigid groups:

| group | delta | members |
|---|---|---|
| up | `+0x43630` | `HeightMaps` `HeightMapsRot` `AngleTable` `SolidityTable` `Map_Sonic` `DPLC_Sonic` `Art_Sonic` |
| down | `−0x1c480` | `Map_Tails` `DPLC_Tails` `Art_Tails` `Map_TailsAppendage` `DPLC_TailsAppendage` `Art_TailsAppendage` `Map_Knuckles` `DPLC_Knuckles` `Art_Knuckles` `Pal_SonicTails` `Pal_Knuckles` |

**Not one symbol below `0x2a3c0` moved** (measured, not assumed: zero moved symbols with an
old-or-new address under `0x2a3c0`, and chunks `00`/`01` of the appendix are byte-identical).

That answers the "is the exact landing construction or coincidence" question: **construction.**
The packing walk lays chained sections down contiguously in the map's declared order, so the
running cursor arriving at that point in the sequence is unchanged when everything before it
is unchanged. `Map_Tails`' section simply took the order slot `HeightMaps`' section vacated,
and both take `align_up(running, 16)` from the same cursor, so both get `0x2a3c0`. It would
have been a coincidence only if the two had different quanta — which is the R7 question, and
is answered next.

## 5. "0 alignment-quantum changes" — re-derived, and it holds

Re-derived from sigil's own tables using `packed_align_of`'s exact rule, not taken from the
aeon report:

```
config_a  86 rows  11 moved   0 quantum changes
config_b  68 rows   2 moved   0
demo      40 rows   0 moved   0
demo_dbg  42 rows   0 moved   0
lean      68 rows  10 moved   0
s4        68 rows  10 moved   0
s4_debug  80 rows  11 moved   0
                  ---------
                  44 moved   0 quantum changes
```

44 and 0 both reproduce. Three things make this a proof rather than a coincidence of
sampling:

1. **`packed_align_of` has exactly one consumer class.** In `packed_true_bases` it is read
   only inside the `labeled[i]` arm (`native.rs:2707`) — a section whose provisional base
   came from a frozen-table row. Label-less sections take the `else` arm and get pure
   contiguity or an island base; no quantum is read for them. The only other call site is
   `seam2::sound_layout`, via `packed_chained_base` on `frozen_prov(…, "Song_MovingTrucks")`
   and `frozen_prov(…, "Sfx_33")` — also frozen rows. **So the quantum set is entirely a
   function of the frozen tables' rows**, and the 44-row sweep is the whole population.
2. **The min-over-labels cannot switch.** A section's provisional base is
   `min over contained frozen labels of (frozen[L] − offset[L])`. `derive_frozen_table`
   writes each row as `s.lma + l.offset`, so *every* frozen label of a section yields exactly
   `s.lma`: the min is degenerate by construction, and a section's base moves by precisely
   its rows' delta.
3. **Every one of the 44 deltas is a multiple of 16.** `packed_align_of` only distinguishes
   residues mod 16, so a base changing by ≡0 (mod 16) cannot change quantum — *regardless of
   the label's offset within its section*. That is stronger than "the addresses are
   16-aligned", which matters, because several moved symbols are not: `Art_Tails` `%16 == 10`,
   `DPLC_Knuckles` `%16 == 6`, `GameState_OJZScroll_Init` `%16 == 4`. Each keeps its residue.

The last point is where the exposure actually sits, and it should not be read as comfort.
Sub-16 quanta are common, not exotic: **14 to 26 rows per shape** sit below 16, down to 2
(`config_b` 22 of 68, `s4_debug` 26 of 80). And one of them is a row this parcel moved:
`GameState_OJZScroll_Init` in `config_a`/`s4_debug`, quantum **4**, `0xa1724 → 0xa60f4` —
the re-pin that is master `b225eddc`'s own subject. It kept its quantum because
`+0x49d0 ≡ 0 (mod 16)`. Nothing enforced that; the parcel happened to move things by bank-
and page-sized amounts and the arithmetic came out clean. A parcel that moves a section by
an amount driven by *content size* has no such guarantee.

## 6. Is either mechanism capable of doing harm?

**The appendix mechanism: no, and it is bounded.** The appendix lives entirely after
`EndOfRom`; it cannot move a single code or data address, and the MD Debugger is its only
reader. Its size varies only with the symbol-to-64 KB-window partition, so the worst case
from the chunk-header term is `2 × 256 = 512` bytes, and from the word-pad term
`1 × non-empty chunks`. Three things are still worth having written down:

- It **does** change the total file size, hence the ROM-end longword at `$1A4` and the
  checksum at `$18E` — both re-folded natively right after the `convsym` call. Any
  size budget, pad-to-power-of-two, or capacity bar measured on the *full* file therefore
  moves for reasons that have nothing to do with the image.
- It is **invisible to every layout-side gate**. `assembled_end` / `anchor_end` were
  unchanged here, so a reviewer reading only the size table sees "nothing moved" and still
  gets a changed `full_crc` and `full_size`. That is precisely what made this two bytes look
  alarming. The only guard on it is the coarse `min_appendix..=0x10000` band in
  `append_deb2_appendix`; `config_b` sits at `0x9949` with ~`0x66b7` of headroom.
- `lean` has **no appendix at all** (file size == `EndOfRom`), so this whole class is simply
  absent from that shape. Worth knowing before generalising a measurement taken on it.

**The R7 mechanism: yes — but only in one direction, and that narrows the inventory's row.**
The inventory states R7 as "moving anything can silently change a section's effective
alignment", which reads as symmetric. Reading the walk, it is not:

- For a **packed** section, `tb = align_up(running, a)` with `a = packed_align_of(prov_old)`.
  The result is divisible by `a`, so `packed_align_of(tb) ≥ a`. The refreeze then records
  `tb` as the new frozen base. **An inferred quantum therefore ratchets upward across
  refreezes and can never silently fall.**
- For an **island**, a **declared anchor**, or a **phase-bank** head, `tb = prov` — the
  quantum is carried through unchanged.
- A **zero-byte marker** (the `EndOfRom` terminus class) has its quantum forced to 2 and
  never reads the inferred one.
- The one way down is a **hand repin** of a frozen row, which is not "silent" in the same
  sense — someone typed it.

So the live hazard class is *not* "a section that needs 16-alignment silently gets 8". It is
**"a quantum silently rises, inserting pad a downstream run did not have, and invalidating
structural pads built against the old value"** — which is exactly what `2c49f538` did when the
SFX pin went `$5BAE8` (quantum 8) → `$5BB10` (quantum 16) and folded every SFX pointer short.
The `[sound.fold-vs-placement]` gate covers the two labels named in R7; the other 20-odd
sub-16 rows per shape remain unguarded, and `GameState_OJZScroll_Init` shows they are not
hypothetical rows in dead corners.

This narrowing is **derived from reading `packed_true_bases`, not measured**. It should be
re-checked before anything is built on it — the walk has four arms and the ratchet argument
depends on the packed arm being the only one that reads the quantum.

## 7. Where this leaves the aeon lane

Nothing to correct. Their "44 rows moved, 0 alignment-quantum changes" is exactly right for
this parcel, and the two-byte growth is not a counter-example to it — it is post-`EndOfRom`
debug data whose size responds to *which 64 KB windows hold symbols*, a quantity no
alignment rule touches. The correct joint statement is:

> The re-layout moved 44 frozen rows across seven shapes, every delta a multiple of 16, so no
> section's inferred alignment quantum changed. `config_b`'s assembled image is the same
> length before and after; its two-byte file growth is entirely in the `convsym` `deb2`
> appendix, which gained one 64 KB chunk-block header when `DPLC_Sonic`/`Art_Sonic` moved
> into `$70000–$7FFFF`. `config_a` and `s4_debug` gained two bytes in the same appendix from
> the same cause in its word-padding term.

## 8. Left open

- **Nothing here was confirmed at runtime.** The claim that the appendix is debugger-only
  data is read off `append_deb2_appendix` and the `ErrorHandlerBlob` contract, not observed.
  TAGGED for the owner's foreground follow-up if it is ever load-bearing.
- **The R7 ratchet argument (§6) is code-derived, not measured.** A red-first probe that
  repins a sub-16 row by a non-16 delta and asserts the resulting pad would settle it. Not
  built here: bar 6 forbids adding a gate incidentally inside a larger task, and this is
  exactly that shape.
- **The `min_appendix..=0x10000` band is the only guard on appendix size**, and it is coarse
  by roughly a factor of two. Whether that is the right bar is a question for whoever owns
  the appendix contract, not a defect found here.

---

*Measured entirely from committed artifacts (`golden/*.bin` and `golden/offcanonical_sizes/*.txt`
at `aa641667` and `b225eddc`) plus `<aeon>/tools/convsym` run on synthetic listings. No build
was required and no emulator was used. Nothing under `crates/sigil-harness/golden/`,
`src/pins.rs` or `repin.toml` was modified.*
