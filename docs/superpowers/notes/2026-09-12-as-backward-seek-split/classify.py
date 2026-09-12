#!/usr/bin/env python3
"""Turn run.sh's RAW.tsv into the per-shape table of the note.

For each assembler and each shape it reads, from that assembler's own image:

  L_next   the longword at $00 (the head table; a word at $00 for c13, the
           Z80-host shape), i.e. where the assembler BOUND the label
  @1234    the offset of the first `12 34` after the head table, i.e. where
           the bytes of the section that label heads actually LANDED
  L_after / @5678  the same pair one section further on

A label and its bytes AGREE when the bound address equals the landing offset.
The verdict compares the two assemblers' whole images, then says which of the
pairs disagree. A shape where asl did not exit 0 is reported as asl-refused
and none of its values are read.

  classify.py <results-dir>
"""
import sys

WORD_HEAD = {"c13_z80_host_cpu_same"}  # Z80 host: `dw` little-endian head words


def read(hexs, shape):
    b = bytes.fromhex(hexs)
    if shape in WORD_HEAD:
        l_next = b[0] | (b[1] << 8)
        l_after = b[2] | (b[3] << 8)
        body = 4
    else:
        l_next = int.from_bytes(b[0:4], "big")
        l_after = int.from_bytes(b[4:8], "big")
        body = 8
    at1234 = b.find(b"\x12\x34", body)
    at5678 = b.find(b"\x56\x78", body)
    return l_next, at1234, l_after, at5678, len(b)


def fmt(v):
    return "-" if v is None or v < 0 else "$%X" % v


def main():
    res = sys.argv[1]
    rows = []
    ended = False
    with open(res + "/RAW.tsv") as f:
        next(f)
        for line in f:
            line = line.rstrip("\n")
            if line == "RUN_END_MARKER":
                ended = True
                continue
            if line.startswith("SHAPES_RUN="):
                continue
            rows.append(line.split("\t"))
    if not ended:
        print("FATAL: RAW.tsv has no RUN_END_MARKER, the run did not finish")
        sys.exit(2)
    print("| shape | asl: L_next / bytes @ | asl: L_after / bytes @ "
          "| sigil: L_next / bytes @ | sigil: L_after / bytes @ | verdict |")
    print("|---|---|---|---|---|---|")
    for shape, arc, diag, ahex, src, shex in rows:
        if arc != "0":
            a = "asl exit %s (%s): not a source of values" % (arc, diag)
            acols = [a, "-"]
            av = None
        else:
            av = read(ahex, shape)
            acols = ["%s / %s" % (fmt(av[0]), fmt(av[1])),
                     "%s / %s" % (fmt(av[2]), fmt(av[3]))]
        if src != "0":
            scols = ["sigil exit %s (refused)" % src, "-"]
            sv = None
        else:
            sv = read(shex, shape)
            scols = ["%s / %s" % (fmt(sv[0]), fmt(sv[1])),
                     "%s / %s" % (fmt(sv[2]), fmt(sv[3]))]
        if av is None:
            verdict = "NO ASL VALUE"
        elif sv is None:
            verdict = "SIGIL REFUSES (asl accepts)"
        elif ahex == shex:
            verdict = "MATCH (images identical, %d bytes)" % av[4]
        else:
            what = []
            if av[0] != sv[0] or av[2] != sv[2]:
                what.append("labels differ")
            else:
                what.append("labels agree")
            if av[1] != sv[1] or av[3] != sv[3]:
                what.append("bytes land elsewhere")
            what.append("image %d vs %d bytes" % (av[4], sv[4]))
            verdict = "DISAGREE: " + ", ".join(what)
        print("| %s | %s | %s | %s | %s | %s |" % (shape, acols[0], acols[1],
                                                  scols[0], scols[1], verdict))
    print("CLASSIFY_ROWS=%d" % len(rows))


if __name__ == "__main__":
    main()
