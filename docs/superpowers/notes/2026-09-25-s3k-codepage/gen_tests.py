#!/usr/bin/env python3
"""gen_tests.py: emit the probe sources and asl's own bytes as Rust consts.

Each accepted probe becomes `const <NAME>_SRC: &str` (the probe file verbatim)
and `const <NAME>_ASL: &str` (the hex of asl's p2bin image from a run that
exited 0). Refused probes become `_SRC` only. Output goes to stdout.
"""
import pathlib, sys

P = pathlib.Path("/home/volence/sonic_hacks/.scratch/s3k-codepage/probes")
accepted = ["cp01", "cp02", "cp03", "cp04", "cp05", "cp06", "cp07", "cp08", "cp09",
            "cp10", "cp11", "cp17", "cp18", "cp19", "cp20", "cp23", "cp24", "cp27",
            "cp29", "cp30", "s3klevsel"]
refused = ["cp12", "cp13", "cp14", "cp15", "cp16", "cp21", "cp22", "cp26", "cp28"]

def lit(s):
    assert '"#' not in s
    return 'r#"' + s + '"#'

for n in accepted:
    out = (P / f"{n}.asl.out").read_text()
    assert "ASL_EXIT=0" in out, n
    src = (P / f"{n}.asm").read_text()
    hexs = (P / f"{n}.asl.bin").read_bytes().hex()
    print(f"const {n.upper()}_SRC: &str = {lit(src)};")
    print(f"const {n.upper()}_ASL: &str = \"{hexs}\";")
for n in refused:
    out = (P / f"{n}.asl.out").read_text()
    assert "ASL_EXIT=2" in out, n
    src = (P / f"{n}.asm").read_text()
    print(f"const {n.upper()}_SRC: &str = {lit(src)};")
