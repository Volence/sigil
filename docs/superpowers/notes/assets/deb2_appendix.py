#!/usr/bin/env python3
"""Decode the convsym `deb2` symbol appendix that sits after `EndOfRom` in a built ROM.

INVESTIGATION ARTIFACT, NOT A GATE. Nothing in the workspace runs this; it exists so
`2026-08-26-config-b-two-byte-growth.md` can be re-measured rather than believed. It
asserts nothing and is wired into no runner — do not read a green run of it as a check
having passed.

The format was reverse-engineered by feeding `<aeon>/tools/convsym` controlled `as_lst`
listings and reading the bytes back, then validated by predicting the byte-exact size of
all twelve shipped appendices (six shapes x two chain entries) from the model below.

    appendix = 4                       magic (de b2) + u16 header
             + 4 * n_chunks            chunk table, one u32 per 64 KB window of the
                                       24-bit address space; 0 = window holds no symbol
             + 4 * n_huffman_leaves    the (u16 code, u8 bit-length, u8 char) code table
             + 2                       the ff ff sentinel before the first block
             + SUM over NON-EMPTY chunks of:
                   2                   the block's own u16 header (blob offset)
                 + 4 * records         (u16 addr-low, u16 byte offset into the blob)
                 + blob                align_up(SUM ceil(bits(name)/8) + 1, 2), except
                                       the final chunk, which takes the +1 guard byte
                                       with no word pad

`n_chunks` is `(highest symbol address >> 16) + 1`, which is 256 for any shape keeping
its `$FFFFxxxx` RAM labels. A chunk table entry points 4 bytes BEFORE its block header.

The consequence the note turns on: record count, name set, and the Huffman code table are
all placement-INDEPENDENT, so the only way placement can change the appendix size is by
changing how the symbols PARTITION across the 64 KB windows.

Usage:  deb2_appendix.py <rom.bin> <EndOfRom-hex>      e.g. ... config_b.bin 0x8b6f0
"""

import struct
import sys


def parse(ap):
    """Split an appendix into (header fields, huffman codes, per-chunk blocks)."""
    header_end = struct.unpack(">H", ap[2:4])[0]
    n_chunks = (header_end - 2) // 4
    table = [struct.unpack(">I", ap[4 + 4 * i : 8 + 4 * i])[0] for i in range(n_chunks)]
    non_empty = [(i, v) for i, v in enumerate(table) if v]

    codes = {}
    for o in range(header_end + 2, non_empty[0][1] + 2, 4):
        codes[(ap[o + 2], struct.unpack(">H", ap[o : o + 2])[0])] = ap[o + 3]

    bounds = [v + 4 for _, v in non_empty] + [len(ap)]
    blocks = []
    for k, (chunk, ptr) in enumerate(non_empty):
        start, end = ptr + 4, bounds[k + 1]
        blob_off = struct.unpack(">H", ap[start : start + 2])[0]
        count = (blob_off - 2) // 4
        recs = [
            (
                struct.unpack(">H", ap[start + 2 + 4 * r : start + 4 + 4 * r])[0],
                struct.unpack(">H", ap[start + 4 + 4 * r : start + 6 + 4 * r])[0],
            )
            for r in range(count)
        ]
        blocks.append((chunk, recs, ap[start + blob_off : end]))
    return n_chunks, len(codes), codes, blocks


def decode_name(blob, byte_off, codes):
    """One Huffman-coded, byte-aligned, NUL-terminated name. Returns (name, bits used)."""
    bit = byte_off * 8
    out = []
    while True:
        code, length = 0, 0
        while True:
            code = (code << 1) | ((blob[bit >> 3] >> (7 - (bit & 7))) & 1)
            length += 1
            bit += 1
            if (length, code) in codes:
                ch = codes[(length, code)]
                break
            if length > 24:
                raise ValueError("no code matched within 24 bits")
        if ch == 0:
            return "".join(out), bit - byte_off * 8
        out.append(chr(ch))


def symbols(ap):
    """Every (address, name) the appendix carries."""
    _, _, codes, blocks = parse(ap)
    for chunk, recs, blob in blocks:
        for addr_lo, off in recs:
            name, _ = decode_name(blob, off, codes)
            yield (chunk << 16) | addr_lo, name


def predicted_size(ap):
    """Rebuild the appendix's length from the model. Equals len(ap) when the model holds."""
    n_chunks, n_leaves, codes, blocks = parse(ap)
    total = 4 + 4 * n_chunks + 4 * n_leaves + 2
    for k, (_, recs, blob) in enumerate(blocks):
        ceil_sum = sum(-(-decode_name(blob, off, codes)[1] // 8) for _, off in recs)
        last = k == len(blocks) - 1
        total += 2 + 4 * len(recs) + ((ceil_sum + 1) if last else -(-(ceil_sum + 1) // 2) * 2)
    return total


def main():
    rom = open(sys.argv[1], "rb").read()
    eor = int(sys.argv[2], 0)
    if eor >= len(rom):
        print("no deb2 appendix: file size == EndOfRom")
        return
    ap = rom[eor:]
    if ap[:2] != b"\xde\xb2":
        raise SystemExit(f"no de b2 magic at {eor:#x}")
    n_chunks, n_leaves, codes, blocks = parse(ap)
    recs = sum(len(r) for _, r, _ in blocks)
    print(f"appendix at {eor:#x}: {len(ap):#x} bytes, predicted {predicted_size(ap):#x}")
    print(f"  chunk table {n_chunks} entries, {len(blocks)} non-empty; "
          f"{n_leaves} huffman leaves; {recs} symbols")
    for chunk, r, blob in blocks:
        print(f"  chunk {chunk:02x}: {len(r):5d} symbols, blob {len(blob):#x}")


if __name__ == "__main__":
    main()
