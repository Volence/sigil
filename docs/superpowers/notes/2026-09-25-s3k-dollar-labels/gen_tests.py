#!/usr/bin/env python3
"""gen_tests.py: emit the probe sources and asl's own bytes as Rust consts.

Each accepted probe becomes `const <NAME>_SRC: &str` (the probe file verbatim) and
`const <NAME>_ASL: &str`, the hex of asl's p2bin image FROM $1200 (every probe
orgs there), from a run that exited 0 with the pass loop complete; the image's
bytes below $1200 are checked to be zero here, and the test checks sigil's are.
Refused probes become `_SRC` only, from a run that exited non-zero.
"""
import pathlib, sys

P = pathlib.Path("/home/volence/sonic_hacks/.scratch/s3k-dollar-labels/probes")
accepted = """d01 d02 d03 d08 d09 e02 e05 e06 e07 e10 e11 e12 e13 e19 f01 f02 f03 f05 f11
f13 f14 f17 f18 f19 f20 g05 g07 g17 i01 i02 i05 i07 i10 i25 i27 i28 i30 i31 j01 j02
j03 j04 j05 j06 k02 k03 k04 k07 y01 y03 y04""".split()
refused = """d04 d05 d06 d07 d10 e01 e03 e08 e14 e15 e18 e20 f07 f08 f09 f10 f12 f15 f16
g01 g02 g04 g06 g10 g11 g12 g13 g14 g16 g20 i16 i26 k01 k06""".split()

def lit(s):
    assert '"#' not in s
    return 'r#"' + s + '"#'

for n in accepted:
    out = (P / f"{n}.asl.out").read_text()
    assert "ASL_EXIT=0" in out and "ASL_DIAG=complete" in out, n
    img = (P / f"{n}.asl.bin").read_bytes()
    assert len(img) > 0x1200 and not any(img[:0x1200]), n
    src = (P / f"{n}.asm").read_text()
    print(f"const {n.upper()}_SRC: &str = {lit(src)};")
    print(f"const {n.upper()}_ASL: &str = \"{img[0x1200:].hex()}\";")
for n in refused:
    out = (P / f"{n}.asl.out").read_text()
    assert "ASL_EXIT=2" in out, n
    src = (P / f"{n}.asm").read_text()
    print(f"const {n.upper()}_SRC: &str = {lit(src)};")
print(f"// {len(accepted)} accepted, {len(refused)} refused", file=sys.stderr)
