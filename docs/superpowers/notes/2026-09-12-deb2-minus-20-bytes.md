# The -20 B on the sonic4 pair at aeon ec640bcf: one debugger symbol lost to an address collision at 0x8000 (2026-09-12)

Asked by the aeon lane for its LS-1a preview. The placement refreeze (chain entry 206, sigil `36aad272`)
shrank `s4` and `s4.debug` by 20 B at aeon `ec640bcf`. Aeon built `df001f99` with the same sigil pair
(`6884bfba`) and saw no size change in any shape. Where did the 20 bytes go, and does it apply at aeon
`f74b7b9c`?

## Answer

**It is specific to the ec640bcf tree and does not apply at `f74b7b9c`.** The 20 B is one symbol record in
the deb2 debugger table that aeon's `convsym` appends after `EndOfRom`: a 4-byte offset entry plus about 16
bytes of encoded name. After the fix, a 68000 code label slid onto exactly `0x8000`, where the listing
already places a Z80-space label, and the table keeps one record per address. Nothing on aeon's current
tree lands on `0x8000`, which is why aeon measured equal symbol counts and equal appendix lengths.

## Measured

Sources: the committed goldens at `36aad272^` (entry 205) and `36aad272` (entry 206), and the refrozen
listings in `~/sonic_hacks/.aeon-sigil-ref` (aeon `ec640bcf`), read 2026-09-12 about 08:50Z.

1. **All 20 B are past `EndOfRom`.** `EndOfRom` is unchanged (s4 `0xBDC92`, s4.debug `0xC14F4`). Appendix
   length, file size minus `EndOfRom`: s4 42867 to 42847, s4.debug 54733 to 54713. demo and demo.debug
   sizes are unchanged.
2. **All 20 B are inside block 0.** The appendix opens with the magic `deb2` and then a 32-bit offset per
   64 KB address block. Block 0 starts at `0x506` in both builds, and every later block's offset is exactly
   `0x14` smaller (s4: `0x65ae` to `0x659a`, `0x89ea` to `0x89d6`, and so on). So blocks 1 and up have
   identical lengths, and no symbol changed block.
3. **Block 0 holds one record fewer.** Its third header word, the offsets-list length, drops by 4 (s4
   `0x128a` to `0x1286`, s4.debug `0x1696` to `0x1692`). Its 4-byte `(offset, name pointer)` records: s4
   1186 to 1185, s4.debug 1446 to 1445. Neither side has two records at one offset. Aligning old to new
   over the slides the fix produced (0 to 24 B), the one old record with no counterpart is at `0x8000` in
   both shapes. Its old neighbours: s4 `0x7ff8` and `0x801e`; s4.debug `0x7ffe` and `0x8038`.
4. **The new listings carry two symbols at `8000`:**
   - `s4.lst`: `$engine.page_cache$PageCache_Prefetch$col_scan : 8000` and `SoundTablesZ80_Head : 8000`
   - `s4.debug.lst`: `$engine.parallax$Parallax_Step4_Fill$anchor_shift_band : 8000` and
     `SoundTablesZ80_Head : 8000`

   `SoundTablesZ80_Head` is a Z80-space label: `PHASE SoundTablesZ80_Head VMA $00008000 LMA $000B8000`.
   The listing gives it its Z80 address. The 68000 label reached `0x8000` through the fix's slide (+8 in s4
   from `0x7ff8`, +2 in s4.debug from `0x7ffe`). demo's listings carry no symbol at `8000`.

**Not measured:** which of the two names `convsym` kept (names are Huffman-coded in the block's heap and
were not decoded), and the keep-one-per-address rule itself, since only `convsym`'s binary is in the
workspace. That rule is inferred from one record fewer with zero duplicate offsets on either side.

## The defect this exposed (sigil, pre-existing)

The listing's symbol table exports a Z80-phase label at its Z80 address, and `convsym` files it in the
68000 debugger table as ROM address `0x8000`. The debugger can therefore name 68000 address `0x8000`
`SoundTablesZ80_Head`, and whenever a real 68000 label lands there, one of the two names is dropped
without a word. The ROM image is unaffected; this is the debug symbol table only. Queue row
`DEB2-Z80-LABEL-IN-68K-TABLE`.

## For aeon

At aeon `f74b7b9c` (`df001f99` plus one docs line), your preview measured equal symbol counts and equal
appendix lengths before and after the new pair. That is the expected result when no 68000 label sits on
`0x8000`, so no size change is expected there. The -20 B was a property of the ec640bcf addresses, not of
the pair. To check any tree, read its listing for more than one symbol at `8000`:
`grep -E ' : 8000 [A-Z] \|' <shape>.lst`.

## Before anyone fixes it

Ranked out of `next` on 2026-09-12 for two reasons, both checkable:

1. **Any repair moves bytes.** The deb2 appendix is inside both debug ROMs, and the provenance chain
   pins the full file (crc32 plus size) for `s4_debug` and `demo_debug`. So a fix is a byte-mover and
   rides the refreeze ritual, including the hand-typed `tests/repin_pins.rs` resync.
2. **It changes a surface the engine lane consumes.** `convsym` reads the listing's symbol table, and
   aeon's own tools read the listing too. Talk to them before changing what the listing exports.

**The first measurement, before any design:** what `asl` itself prints in its listing's symbol table
for a label inside a `PHASE` block. If `asl` also lists it at its phase (Z80) address, sigil is being
compatible and the collision is `convsym`'s, so the fix may belong on the consuming side. Read
`docs/OVERSEER-REFERENCE.md`, "Selecting and citing the `asl` oracle", before invoking it.
