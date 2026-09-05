#!/usr/bin/env python3
"""Capstone MC68000 disassembly dump, the raw side of sigil's differential gate.

This script makes NO judgement. It disassembles byte buffers with capstone in
`CS_ARCH_M68K` / `CS_MODE_M68K_000` and prints what capstone said, verbatim.
Every classification, normalisation and comparison lives on the Rust side
(`crates/sigil-isa/tests/m68k_capstone_differential.rs`), so the oracle's
answer can be read and audited without reasoning about this file.

Two input modes:

  sweep            emit one line per opcode word 0000..FFFF, each padded to
                   PAD_LEN bytes with the extension-word pattern given by
                   --pad2=0xHHLL (default 0x0000), repeated.
  bytes            read hex-encoded buffers from stdin, one per line, and emit
                   one line per input line in input order.

Output is TSV, one record per input, on stdout:

  <key> \t ok    \t <len> \t <mnemonic> \t <op_str>
  <key> \t reject

`<key>` is the four-hex-digit opcode word in sweep mode and the input hex
string in bytes mode. `reject` covers both "capstone consumed nothing" and
"capstone produced its `dc.w` data placeholder" (instruction id 0), the two
ways capstone says a word is not an MC68000 instruction.

The first stdout line is a banner: `#capstone <python-binding-version> <core>`.
Import failure exits 3 with a message on stderr; the caller turns that into a
gate failure, never a skip.

Addresses: every buffer is disassembled at base address 0, so capstone's
resolved branch / PC-relative targets are pure offsets from the start of the
instruction and the Rust side can re-derive them from sigil's displacement
without knowing a base.
"""

import sys

PAD_LEN = 14  # sigil's longest emitted form is 10 bytes; pad generously.


def main() -> int:
    try:
        import capstone
        from capstone import Cs, CS_ARCH_M68K, CS_MODE_M68K_000
    except Exception as exc:  # noqa: BLE001 - any import problem is fatal here
        print(f"capstone import failed: {exc}", file=sys.stderr)
        return 3

    mode = sys.argv[1] if len(sys.argv) > 1 else "sweep"
    pad_word = 0
    for arg in sys.argv[2:]:
        if arg.startswith("--pad2="):
            pad_word = int(arg.split("=", 1)[1], 0) & 0xFFFF
        else:
            print(f"unknown argument {arg!r}", file=sys.stderr)
            return 2

    md = Cs(CS_ARCH_M68K, CS_MODE_M68K_000)
    md.detail = False
    md.skipdata = False

    core = ".".join(str(n) for n in capstone.cs_version())
    out = sys.stdout
    out.write(f"#capstone {capstone.__version__} {core}\n")

    def emit(key: str, buf: bytes) -> None:
        got = None
        for insn in md.disasm(buf, 0):
            got = insn
            break
        if got is None or got.id == 0:
            out.write(f"{key}\treject\n")
        else:
            out.write(f"{key}\tok\t{got.size}\t{got.mnemonic}\t{got.op_str}\n")

    if mode == "sweep":
        tail = (pad_word.to_bytes(2, "big") * (PAD_LEN // 2))[: PAD_LEN - 2]
        for w in range(0x10000):
            emit(f"{w:04X}", w.to_bytes(2, "big") + tail)
    elif mode == "bytes":
        for line in sys.stdin:
            key = line.strip()
            if not key:
                continue
            emit(key, bytes.fromhex(key))
    else:
        print(f"unknown mode {mode!r}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
