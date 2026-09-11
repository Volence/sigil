#!/usr/bin/env python3
"""gen_test.py <template.rs> <out.rs> <listings-extract.txt>

Build the PROBES table of as_z80_half_registers.rs from the probe rounds
m1..m4 in probes/, each probe's source and the pinned asl's listing, and write
an extract of every listing's code lines and footer (the provenance of each
row). Values come only from ASL_EXIT=0 runs with a complete pass footer.
"""
import os
import re
import sys

S = "/home/volence/sonic_hacks/.scratch/z80-half-registers"
ROUNDS = ["m1", "m2", "m3", "m4"]
ASL_MD5 = "61e672562465725a8c102288a7da9098"

# Probes where sigil differs from asl for reasons that are not the halves;
# listed in the note as open, left out of the table by name.
EXCLUDE = {
    "m1/ctl_sll_a": "sll is not implemented (loud)",
    "m1/ctl_sll_b": "sll is not implemented (loud)",
    "m2/alu1r_add": "one-operand add on a plain register: sigil accepts, asl refuses",
    "m2/alu1r_add_z80": "one-operand add on a plain register: sigil accepts, asl refuses",
    "m2/alu1r_adc": "one-operand adc on a plain register: sigil accepts, asl refuses",
    "m2/alu1r_adc_z80": "one-operand adc on a plain register: sigil accepts, asl refuses",
    "m2/alu1r_sbc": "one-operand sbc on a plain register: sigil accepts, asl refuses",
    "m2/alu1r_sbc_z80": "one-operand sbc on a plain register: sigil accepts, asl refuses",
    "m2/alu1i_add": "one-operand add on an immediate: sigil accepts, asl refuses",
    "m2/alu1i_adc": "one-operand adc on an immediate: sigil accepts, asl refuses",
    "m2/alu1i_sbc": "one-operand sbc on an immediate: sigil accepts, asl refuses",
    "m2/mix_LD_A_IXL": "upper-case documented register A: sigil refuses (loud)",
    "m2/mix_ld_A_ixl": "upper-case documented register A: sigil refuses (loud)",
    "m2/mix_ld_IXU_B": "upper-case documented register B: sigil refuses (loud)",
    "m2/mix_ld_ixl_A": "upper-case documented register A: sigil refuses (loud)",
    "m2/mode_68k_save_undoc": "a save with no restore: asl refuses (#1460), sigil accepts",
}

LINE = re.compile(r"^\s+\d+/\s*[0-9A-F]+ :((?: (?:[0-9A-F]{2})+(?= |\t|$))*)", re.M)


def rust_str(s):
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"').replace("\t", "\\t").replace("\n", "\\n") + '"'


def main():
    template, out, extract = sys.argv[1:4]
    rows, ext = [], []
    excluded_seen = set()
    for rnd in ROUNDS:
        d = os.path.join(S, "probes", rnd)
        runlog = open(os.path.join(d, "_run.log")).read()
        md5 = re.search(r"ASL_MD5=(\w+)", runlog).group(1)
        assert md5 == ASL_MD5, (rnd, md5)
        exits = re.findall(r"== (\S+)\.asm ASL_EXIT=(\d+)", runlog)
        for name, rc in exits:
            key = f"{rnd}/{name}"
            src = open(os.path.join(d, name + ".asm")).read()
            txt = open(os.path.join(d, name + ".lst"), errors="replace").read()
            complete = bool(re.search(r"^ +\d+ passe?s?$", txt, re.M)) and \
                not re.search(r"^\s+Additional necessary passes", txt, re.M)
            code = [l for l in txt.splitlines() if re.match(r"^\s+\d+/\s*[0-9A-F]+ :", l)]
            foot = [l.strip() for l in txt.splitlines() if re.match(r"^\s+\d+ (passe?s?|errors?|warnings?)$", l)]
            ext.append(f"== {key} ASL_EXIT={rc}\n" + "\n".join(code) + "\n  [" + ", ".join(foot) + "]")
            if key in EXCLUDE:
                excluded_seen.add(key)
                continue
            if int(rc) == 0:
                assert complete, key
                bs = []
                for m in LINE.finditer(txt):
                    for tok in m.group(1).split():
                        bs += [tok[k:k + 2] for k in range(0, len(tok), 2)]
                val = "Some(&[" + ", ".join("0x" + b for b in bs) + "])"
            else:
                val = "None"
            rows.append(f"    ({rust_str(key)}, {rust_str(src)}, {val}),")
    assert excluded_seen == set(EXCLUDE), set(EXCLUDE) - excluded_seen
    t = open(template).read()
    assert t.count("// @PROBES@\n") == 1
    open(out, "w").write(t.replace("// @PROBES@\n", "\n".join(rows) + "\n"))
    with open(extract, "w") as fh:
        fh.write(f"# every probe's listing: code lines and pass footer. asl md5 {ASL_MD5}\n")
        fh.write("\n".join(ext) + "\n")
    print(f"{len(rows)} probes in the table, {len(EXCLUDE)} excluded by name, {len(ext)} listings extracted")


main()
